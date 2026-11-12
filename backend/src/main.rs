use std::{env, sync::Arc, collections::HashSet};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use axum::{body::Body, extract::{Path, Query, State}, http::{header, HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::{get, post, patch}, Json, Router};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use uuid::Uuid;

#[derive(Clone)] struct App { db: PgPool, secret: Arc<String>, doc_url: String, client: reqwest::Client }
#[derive(Debug)] struct ErrApi(StatusCode, String);
impl IntoResponse for ErrApi { fn into_response(self)->Response { (self.0, Json(json!({"error":self.1}))).into_response() } }
type ResultApi<T> = Result<T,ErrApi>;
fn bad(s: impl ToString)->ErrApi { ErrApi(StatusCode::BAD_REQUEST,s.to_string()) }
fn db(e:sqlx::Error)->ErrApi { ErrApi(StatusCode::INTERNAL_SERVER_ERROR,{ tracing::error!("Database error: {e}"); "Database operation failed".to_string() }) }
fn forbidden()->ErrApi { ErrApi(StatusCode::FORBIDDEN,"Nedovoljna ovlast".into()) }
fn unauthorized()->ErrApi { ErrApi(StatusCode::UNAUTHORIZED,"Prijava je potrebna".into()) }
#[derive(Debug, Serialize, Deserialize)] struct Claims { sub:Uuid, role:String, department_id:Option<Uuid>, #[serde(default)] sector_id:Option<Uuid>, exp:usize }
async fn who(s:&App,h:&HeaderMap)->ResultApi<Claims> {
 let token=h.get(header::AUTHORIZATION).and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).ok_or_else(unauthorized)?;
 let mut c=decode::<Claims>(token,&DecodingKey::from_secret(s.secret.as_bytes()),&Validation::default()).map_err(|_|unauthorized())?.claims;
 // Always use current database role/scope: deactivation and edits revoke access immediately.
 let record=sqlx::query("SELECT role,sector_id,department_id FROM users WHERE id=$1 AND active=true")
  .bind(c.sub).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(unauthorized)?;
 c.role=record.get("role");c.sector_id=record.get("sector_id");c.department_id=record.get("department_id");
 Ok(c)
}
fn org_admin(c:&Claims)->bool {matches!(c.role.as_str(),"admin"|"org_admin")}
fn director(c:&Claims)->bool {c.role=="director"&&c.sector_id.is_some()}
async fn guard_dept(s:&App,c:&Claims,dept:Uuid,action:&str)->ResultApi<()> {
 if matches!(c.role.as_str(),"admin"|"org_admin"){return Err(forbidden());}
 let sector:Option<Uuid>=sqlx::query_scalar("SELECT sector_id FROM departments WHERE id=$1")
   .bind(dept).fetch_optional(&s.db).await.map_err(db)?;
 let sector=sector.ok_or_else(forbidden)?;
 let permitted=match c.role.as_str(){
  "director"=>c.sector_id==Some(sector)&&action=="read",
  "manager"=>c.sector_id==Some(sector)&&c.department_id==Some(dept),
  "employee"=>c.sector_id==Some(sector)&&c.department_id==Some(dept)&&action!="manage",
  _=>false
 };
 if permitted {Ok(())}else{Err(forbidden())}
}
fn role_scope(role:&str,sector:Option<Uuid>,department:Option<Uuid>)->bool {
 match role {
  "admin"|"org_admin"=>sector.is_none()&&department.is_none(),
  "director"=>sector.is_some()&&department.is_none(),
  "manager"|"employee"=>sector.is_some()&&department.is_some(),
  _=>false
 }
}
async fn audit(s:&App,c:&Claims,action:&str,id:Option<Uuid>) { let _=sqlx::query("INSERT INTO audit_events(user_id,action,entity_id) VALUES($1,$2,$3)").bind(c.sub).bind(action).bind(id).execute(&s.db).await; }
fn hash(p:&str)->ResultApi<String> { let salt=SaltString::generate(&mut OsRng); Ok(Argon2::default().hash_password(p.as_bytes(),&salt).map_err(|e|bad(e.to_string()))?.to_string()) }

#[derive(Deserialize)] struct Login { username:String,password:String }
async fn login(State(s):State<App>,Json(p):Json<Login>)->ResultApi<Json<Value>> {
 let r=sqlx::query("SELECT id,username,full_name,role,department_id,sector_id,password_hash FROM users WHERE username=$1 AND active=true").bind(&p.username).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(unauthorized)?;
 let pass:String=r.get("password_hash");
 let valid=PasswordHash::new(&pass).ok().map(|ph|Argon2::default().verify_password(p.password.as_bytes(),&ph).is_ok()).unwrap_or(false);
 if !valid {return Err(unauthorized());}
 let c=Claims{sub:r.get("id"),role:r.get("role"),department_id:r.get("department_id"),sector_id:r.get("sector_id"),exp:(Utc::now().timestamp()+60*60*12) as usize};
 let token=encode(&Header::default(),&c,&EncodingKey::from_secret(s.secret.as_bytes())).map_err(|e|bad(e.to_string()))?;
 Ok(Json(json!({"token":token,"user":{"id":c.sub,"username":p.username,"name":r.get::<String,_>("full_name"),"role":c.role,"department_id":c.department_id,"sector_id":c.sector_id}})))
}
async fn me(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let r=sqlx::query("SELECT username,full_name FROM users WHERE id=$1 AND active=true").bind(c.sub).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(unauthorized)?;
 Ok(Json(json!({"id":c.sub,"username":r.get::<String,_>("username"),"name":r.get::<String,_>("full_name"),"role":c.role,"department_id":c.department_id,"sector_id":c.sector_id})))
}

async fn sectors(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let rows=if org_admin(&c) {
  sqlx::query("SELECT id,code,name,created_at FROM sectors ORDER BY name").fetch_all(&s.db).await.map_err(db)?
 }else{
  let id=c.sector_id.ok_or_else(forbidden)?;
  sqlx::query("SELECT id,code,name,created_at FROM sectors WHERE id=$1 ORDER BY name").bind(id).fetch_all(&s.db).await.map_err(db)?
 };
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"code":r.get::<String,_>("code"),"name":r.get::<String,_>("name")})).collect())))
}
#[derive(Deserialize)] struct NewSector {code:String,name:String}
async fn create_sector(State(s):State<App>,h:HeaderMap,Json(p):Json<NewSector>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;if !org_admin(&c){return Err(forbidden());}
 let code=p.code.trim().to_uppercase();
 if code.is_empty()||code.len()>20||p.name.trim().is_empty()||p.name.len()>200{return Err(bad("Nevaljan kod ili naziv sektora"));}
 let id:Uuid=sqlx::query_scalar("INSERT INTO sectors(code,name) VALUES($1,$2) RETURNING id")
 .bind(code).bind(p.name.trim()).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"sector.create",Some(id)).await;Ok(Json(json!({"id":id})))
}
async fn departments(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let rows=if org_admin(&c) {
  sqlx::query("SELECT id,code,name,sort_order,sector_id FROM departments ORDER BY sector_id,sort_order,name")
   .fetch_all(&s.db).await.map_err(db)?
 }else{
  let sector=c.sector_id.ok_or_else(forbidden)?;
  sqlx::query("SELECT id,code,name,sort_order,sector_id FROM departments WHERE sector_id=$1 ORDER BY sort_order,name")
   .bind(sector).fetch_all(&s.db).await.map_err(db)?
 };
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"code":r.get::<String,_>("code"),"name":r.get::<String,_>("name"),"sort_order":r.get::<i32,_>("sort_order"),"sector_id":r.get::<Uuid,_>("sector_id")})).collect())))
}
#[derive(Deserialize)] struct NewDepartment {sector_id:Uuid,code:String,name:String}
async fn create_department(State(s):State<App>,h:HeaderMap,Json(p):Json<NewDepartment>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;if !org_admin(&c){return Err(forbidden());}
 if p.code.trim().is_empty()||p.code.len()>20||p.name.trim().is_empty()||p.name.len()>200{return Err(bad("Nevaljan kod ili naziv odjela"));}
 let id:Uuid=sqlx::query_scalar("INSERT INTO departments(sector_id,code,name) VALUES($1,$2,$3) RETURNING id")
  .bind(p.sector_id).bind(p.code.trim().to_uppercase()).bind(p.name.trim()).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"department.create",Some(id)).await;Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)] struct NewUser { username:String,full_name:String,password:String,role:String,sector_id:Option<Uuid>,department_id:Option<Uuid> }
async fn validate_target_user(s:&App,p_role:&str,p_sector:Option<Uuid>,p_dept:Option<Uuid>)->ResultApi<()> {
 if !role_scope(p_role,p_sector,p_dept) {return Err(bad("Uloga nije usklađena sa sektorom i odjelom"));}
 if let Some(sector)=p_sector {
  let found:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sectors WHERE id=$1)").bind(sector).fetch_one(&s.db).await.map_err(db)?;
  if !found{return Err(bad("Sektor ne postoji"));}
 }
 if let Some(dept)=p_dept {
  let linked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM departments WHERE id=$1 AND sector_id=$2)")
  .bind(dept).bind(p_sector).fetch_one(&s.db).await.map_err(db)?;
  if !linked{return Err(bad("Odjel nije u odabranom sektoru"));}
 }
 Ok(())
}
async fn create_user(State(s):State<App>,h:HeaderMap,Json(p):Json<NewUser>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;if !org_admin(&c){return Err(forbidden());}
 if p.role=="admin"&&c.role!="admin"{return Err(forbidden());}
 if p.password.len()<10||p.username.trim().is_empty()||p.full_name.trim().is_empty(){return Err(bad("Unesi korisničko ime, ime i lozinku od najmanje 10 znakova"));}
 validate_target_user(&s,&p.role,p.sector_id,p.department_id).await?;
 let id:Uuid=sqlx::query_scalar("INSERT INTO users(username,full_name,password_hash,role,sector_id,department_id) VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
  .bind(p.username.trim()).bind(p.full_name.trim()).bind(hash(&p.password)?).bind(&p.role).bind(p.sector_id).bind(p.department_id).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"user.create",Some(id)).await;Ok(Json(json!({"id":id})))
}
async fn users(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;if !org_admin(&c){return Err(forbidden());}
 let rows=sqlx::query("SELECT id,username,full_name,role,sector_id,department_id,active,created_at FROM users ORDER BY full_name")
 .fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({
  "id":r.get::<Uuid,_>("id"),"username":r.get::<String,_>("username"),"full_name":r.get::<String,_>("full_name"),
  "role":r.get::<String,_>("role"),"sector_id":r.get::<Option<Uuid>,_>("sector_id"),
  "department_id":r.get::<Option<Uuid>,_>("department_id"),"active":r.get::<bool,_>("active"),
  "created_at":r.get::<chrono::DateTime<Utc>,_>("created_at")
 })).collect())))
}
#[derive(Deserialize)] struct UpdateUser {username:String,full_name:String,role:String,sector_id:Option<Uuid>,department_id:Option<Uuid>,active:bool,password:Option<String>}
async fn update_user(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<UpdateUser>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;if !org_admin(&c){return Err(forbidden());}
 let previous=sqlx::query("SELECT role,active FROM users WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Korisnik ne postoji"))?;
 let old_role:String=previous.get("role");
 if (p.role=="admin"||old_role=="admin")&&c.role!="admin"{return Err(forbidden());}
 if id==c.sub&&(p.role!=c.role||!p.active){return Err(bad("Ne možeš sebi ukloniti administratorsku ulogu ili deaktivirati račun"));}
 if p.full_name.trim().is_empty()||p.username.trim().is_empty()||p.username.trim().len()>100{return Err(bad("Korisničko ime i ime su obvezni"));}
 if let Some(pass)=&p.password {if !pass.is_empty()&&pass.len()<10{return Err(bad("Nova lozinka mora imati najmanje 10 znakova"));}}
 validate_target_user(&s,&p.role,p.sector_id,p.department_id).await?;
 let pwd=match p.password.as_deref(){Some(value) if !value.is_empty()=>Some(hash(value)?),_=>None};
 sqlx::query("UPDATE users SET full_name=$2,role=$3,sector_id=$4,department_id=$5,active=$6,password_hash=COALESCE($7,password_hash),username=$8 WHERE id=$1")
  .bind(id).bind(p.full_name.trim()).bind(&p.role).bind(p.sector_id).bind(p.department_id).bind(p.active).bind(pwd).bind(p.username.trim())
  .execute(&s.db).await.map_err(db)?;
 audit(&s,&c,"user.update",Some(id)).await;Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)] struct DashboardFilter { year:i32, month:i32 }
fn dashboard_period_valid(year:i32,month:i32)->bool { (2000..=2100).contains(&year) && (1..=12).contains(&month) }
async fn dashboard(State(s):State<App>,h:HeaderMap,Query(p):Query<DashboardFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 if !dashboard_period_valid(p.year,p.month) {
  return Err(bad("Nevaljan mjesec ili godina"));
 }
 if !matches!(c.role.as_str(),"director"|"manager"|"employee"){return Err(forbidden());}
 let scope_sector=c.sector_id.ok_or_else(forbidden)?;
 let scoped_dept=if director(&c) {None} else {Some(c.department_id.ok_or_else(forbidden)?)};
 // Counts are sourced from distinct activities, never from a join with multiple updates.
 let departments=sqlx::query(r#"
 SELECT d.id,d.name,d.code,
   (SELECT COUNT(*) FROM activities a WHERE a.department_id=d.id) AS total,
   (SELECT COUNT(*) FROM activities a WHERE a.department_id=d.id AND a.status='in_progress') AS active,
   (SELECT COUNT(*) FROM activities a WHERE a.department_id=d.id AND a.status='completed') AS completed,
   (SELECT COUNT(*) FROM activity_updates u JOIN activities a ON a.id=u.activity_id
     WHERE a.department_id=d.id AND u.year=$2 AND u.month=$3 AND u.kind='report') AS monthly_updates,
   (SELECT COUNT(*) FROM activity_updates u JOIN activities a ON a.id=u.activity_id
     WHERE a.department_id=d.id AND u.year=$2 AND u.month=$3 AND u.kind='report'
       AND u.progress='completed') AS monthly_completed,
   COALESCE((SELECT dd.status FROM department_documents dd
      WHERE dd.department_id=d.id AND dd.year=$2 AND dd.month=$3 AND dd.kind='report'),'draft') AS report_status,
   COALESCE((SELECT dd.status FROM department_documents dd
      WHERE dd.department_id=d.id AND dd.year=$2 AND dd.month=$3 AND dd.kind='plan'),'draft') AS plan_status
 FROM departments d WHERE ($1::uuid IS NULL OR d.id=$1) AND d.sector_id=$4
 ORDER BY d.sort_order,d.name
 "#).bind(scoped_dept).bind(p.year).bind(p.month).bind(scope_sector).fetch_all(&s.db).await.map_err(db)?;
 let recent=sqlx::query(r#"
 SELECT u.activity_id,a.title,d.name AS department_name,u.year,u.month,u.kind,
        u.content,u.progress,u.updated_at
 FROM activity_updates u JOIN activities a ON a.id=u.activity_id
 JOIN departments d ON d.id=a.department_id
 WHERE ($1::uuid IS NULL OR d.id=$1) AND d.sector_id=$2
 ORDER BY u.updated_at DESC,u.id DESC LIMIT 8
 "#).bind(scoped_dept).bind(scope_sector).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(json!({
  "year":p.year,"month":p.month,
  "departments":departments.iter().map(|r|json!({
   "id":r.get::<Uuid,_>("id"),"name":r.get::<String,_>("name"),"code":r.get::<String,_>("code"),
   "total":r.get::<i64,_>("total"),"active":r.get::<i64,_>("active"),
   "completed":r.get::<i64,_>("completed"),
   "monthly_updates":r.get::<i64,_>("monthly_updates"),
   "monthly_completed":r.get::<i64,_>("monthly_completed"),
   "report_status":r.get::<String,_>("report_status"),
   "plan_status":r.get::<String,_>("plan_status")
  })).collect::<Vec<_>>(),
  "recent":recent.iter().map(|r|json!({
   "activity_id":r.get::<Uuid,_>("activity_id"),"title":r.get::<String,_>("title"),
   "department_name":r.get::<String,_>("department_name"),
   "year":r.get::<i32,_>("year"),"month":r.get::<i32,_>("month"),
   "kind":r.get::<String,_>("kind"),
   "content":r.get::<String,_>("content"),"progress":r.get::<String,_>("progress"),
   "updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at")
  })).collect::<Vec<_>>()
 })))
}

#[derive(Deserialize)] struct FilterDept {department_id:Uuid}
async fn activities(State(s):State<App>,h:HeaderMap,Query(p):Query<FilterDept>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"read").await?;
 let rows=sqlx::query("SELECT a.id,a.department_id,a.parent_id,a.title,a.description,a.type,a.status,a.sort_order,a.created_by,a.created_at,a.updated_at,u.full_name AS created_by_name FROM activities a LEFT JOIN users u ON u.id=a.created_by WHERE a.department_id=$1 ORDER BY a.sort_order,a.created_at").bind(p.department_id).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"department_id":r.get::<Uuid,_>("department_id"),"parent_id":r.get::<Option<Uuid>,_>("parent_id"),"title":r.get::<String,_>("title"),"description":r.get::<String,_>("description"),"type":r.get::<String,_>("type"),"status":r.get::<String,_>("status"),"sort_order":r.get::<i32,_>("sort_order"),"created_by":r.get::<Option<Uuid>,_>("created_by"),"created_by_name":r.get::<Option<String>,_>("created_by_name"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at")})).collect())))
}
#[derive(Deserialize)]
struct ActivitySearchFilter {
 q:String,
 department_id:Option<Uuid>,
 year:Option<i32>,
 month:Option<i32>,
 kind:Option<String>,
 limit:Option<i64>,
}
fn search_pattern(term:&str)->String {
 format!("%{}%",term.replace('\\',"\\\\").replace('%',"\\%").replace('_',"\\_"))
}
async fn search_activities(State(s):State<App>,h:HeaderMap,Query(p):Query<ActivitySearchFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let term=p.q.trim();
 if term.chars().count()<2 || term.chars().count()>120 {return Err(bad("Pretraga zahtijeva 2 do 120 znakova"));}
 if let Some(y)=p.year {if !(2000..=2100).contains(&y){return Err(bad("Nevaljana godina"));}}
 if let Some(m)=p.month {if !(1..=12).contains(&m){return Err(bad("Nevaljan mjesec"));}}
 if let Some(k)=&p.kind {if !["plan","report"].contains(&k.as_str()){return Err(bad("Nevaljana vrsta dokumenta"));}}
 if !matches!(c.role.as_str(),"director"|"manager"|"employee"){return Err(forbidden());}
 let sector=c.sector_id.ok_or_else(forbidden)?;
 if let Some(requested)=p.department_id {guard_dept(&s,&c,requested,"read").await?;}
 let scope=if director(&c){p.department_id}else{
  let d=c.department_id.ok_or_else(forbidden)?;
  if p.department_id.is_some()&&p.department_id!=Some(d){return Err(forbidden());}Some(d)
 };
 let pattern=search_pattern(term);
 let rows=sqlx::query(r#"
 SELECT a.id AS activity_id,a.department_id,d.name AS department_name,
        a.title,a.description,a.type,a.status,
        u.year,u.month,u.kind,u.content,u.progress,u.include_in_document,u.updated_at
 FROM activities a
 JOIN departments d ON d.id=a.department_id
 LEFT JOIN activity_updates u ON u.activity_id=a.id
 WHERE ($1::uuid IS NULL OR a.department_id=$1) AND d.sector_id=$7
   AND (a.title ILIKE $2 ESCAPE '\' OR a.description ILIKE $2 ESCAPE '\'
        OR u.content ILIKE $2 ESCAPE '\')
   AND ($3::int IS NULL OR u.year=$3)
   AND ($4::int IS NULL OR u.month=$4)
   AND ($5::text IS NULL OR u.kind=$5)
 ORDER BY a.title,u.year DESC NULLS LAST,u.month DESC NULLS LAST,u.kind
 LIMIT $6
 "#)
 .bind(scope).bind(pattern).bind(p.year).bind(p.month).bind(p.kind)
 .bind(p.limit.unwrap_or(50).clamp(1,100)).bind(sector)
 .fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({
  "activity_id":r.get::<Uuid,_>("activity_id"),
  "department_id":r.get::<Uuid,_>("department_id"),
  "department_name":r.get::<String,_>("department_name"),
  "title":r.get::<String,_>("title"),
  "description":r.get::<String,_>("description"),
  "type":r.get::<String,_>("type"),
  "status":r.get::<String,_>("status"),
  "year":r.get::<Option<i32>,_>("year"),
  "month":r.get::<Option<i32>,_>("month"),
  "kind":r.get::<Option<String>,_>("kind"),
  "content":r.get::<Option<String>,_>("content"),
  "progress":r.get::<Option<String>,_>("progress"),
  "include_in_document":r.get::<Option<bool>,_>("include_in_document"),
  "updated_at":r.get::<Option<chrono::DateTime<Utc>>,_>("updated_at")
 })).collect())))
}
async fn activity_history(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let a=sqlx::query("SELECT a.id,a.department_id,d.name AS department_name,a.parent_id,a.title,a.description,a.type,a.status,a.created_at,a.updated_at,ac.full_name AS created_by_name FROM activities a JOIN departments d ON d.id=a.department_id LEFT JOIN users ac ON ac.id=a.created_by WHERE a.id=$1")
  .bind(id).fetch_optional(&s.db).await.map_err(db)?
  .ok_or_else(||ErrApi(StatusCode::NOT_FOUND,"Aktivnost nije pronađena".into()))?;
 let dept:Uuid=a.get("department_id");
 guard_dept(&s,&c,dept,"read").await?;
 let rows=sqlx::query(r#"
 SELECT u.id,u.year,u.month,u.kind,u.content,u.progress,u.include_in_document,u.created_at,u.updated_at,u.creation_time_estimated,
        usr.full_name AS updated_by_name,uc.full_name AS created_by_name
 FROM activity_updates u
 LEFT JOIN users usr ON usr.id=u.updated_by
 LEFT JOIN users uc ON uc.id=u.created_by
 WHERE u.activity_id=$1
 ORDER BY u.year DESC,u.month DESC,u.kind
 "#).bind(id).fetch_all(&s.db).await.map_err(db)?;
 let entries:Vec<Value>=rows.iter().map(|r|json!({
  "id":r.get::<Uuid,_>("id"),
  "year":r.get::<i32,_>("year"),
  "month":r.get::<i32,_>("month"),
  "kind":r.get::<String,_>("kind"),
  "content":r.get::<String,_>("content"),
  "progress":r.get::<String,_>("progress"),
  "include_in_document":r.get::<bool,_>("include_in_document"),
  "updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at"),
  "created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),
  "creation_time_estimated":r.get::<bool,_>("creation_time_estimated"),
  "created_by_name":r.get::<Option<String>,_>("created_by_name"),
  "updated_by_name":r.get::<Option<String>,_>("updated_by_name")
 })).collect();
 Ok(Json(json!({"activity":{
  "id":a.get::<Uuid,_>("id"),"department_id":dept,
  "department_name":a.get::<String,_>("department_name"),
  "parent_id":a.get::<Option<Uuid>,_>("parent_id"),
  "title":a.get::<String,_>("title"),"description":a.get::<String,_>("description"),
  "type":a.get::<String,_>("type"),"status":a.get::<String,_>("status"),"created_at":a.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":a.get::<chrono::DateTime<Utc>,_>("updated_at"),"created_by_name":a.get::<Option<String>,_>("created_by_name")
 },"entries":entries})))
}

#[derive(Deserialize)] struct NewActivity { department_id:Uuid,parent_id:Option<Uuid>,title:String,description:Option<String>,#[serde(rename="type")] activity_type:String,sort_order:Option<i32> }
async fn create_activity(State(s):State<App>,h:HeaderMap,Json(p):Json<NewActivity>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"write").await?;
 if p.title.trim().is_empty() || !["ongoing","project","one_off"].contains(&p.activity_type.as_str()) {return Err(bad("Naziv ili vrsta aktivnosti nije valjana"));}
 if let Some(parent)=p.parent_id { let d:Option<Uuid>=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(parent).fetch_optional(&s.db).await.map_err(db)?; if d!=Some(p.department_id) {return Err(bad("Podstavka mora biti iz istog odjela"));} }
 let id:Uuid=sqlx::query_scalar("INSERT INTO activities(department_id,parent_id,title,description,type,sort_order,created_by) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id").bind(p.department_id).bind(p.parent_id).bind(p.title).bind(p.description.unwrap_or_default()).bind(p.activity_type).bind(p.sort_order.unwrap_or(0)).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"activity.create",Some(id)).await; Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)] struct EditActivity {title:String,description:String,status:String,sort_order:i32}
async fn edit_activity(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<EditActivity>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 if p.title.trim().is_empty()||p.title.len()>1000||p.description.len()>12000 {return Err(bad("Nevaljan naslov ili opis"));}
 if !["in_progress","completed","paused","cancelled"].contains(&p.status.as_str()){return Err(bad("Nevaljan status"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 let row=sqlx::query("SELECT department_id,created_by,title,description FROM activities WHERE id=$1 FOR UPDATE")
  .bind(id).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 let dept:Uuid=row.get("department_id");let owner:Option<Uuid>=row.get("created_by");
 guard_dept(&s,&c,dept,"write").await?;if c.role=="employee"&&owner!=Some(c.sub){return Err(forbidden());}
 let old_title:String=row.get("title");let old_description:String=row.get("description");
 if old_title!=p.title||old_description!=p.description {
  sqlx::query("INSERT INTO activity_field_changes(activity_id,changed_by,old_title,new_title,old_description,new_description) VALUES($1,$2,$3,$4,$5,$6)")
   .bind(id).bind(c.sub).bind(&old_title).bind(&p.title).bind(&old_description).bind(&p.description).execute(&mut *tx).await.map_err(db)?;
 }
 sqlx::query("UPDATE activities SET title=$2,description=$3,status=$4,sort_order=$5,updated_at=now() WHERE id=$1")
  .bind(id).bind(&p.title).bind(&p.description).bind(&p.status).bind(p.sort_order).execute(&mut *tx).await.map_err(db)?;
 tx.commit().await.map_err(db)?;
 audit(&s,&c,"activity.update",Some(id)).await;Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)] struct RenameActivity {title:String,description:String}
async fn rename_activity(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<RenameActivity>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 if p.title.trim().is_empty() || p.title.len()>1000 || p.description.len()>12000 {
  return Err(bad("Naziv je obavezan i mora biti kraći od 1000 znakova, opis od 12000 znakova"));
 }
 let mut tx=s.db.begin().await.map_err(db)?;
 let row=sqlx::query("SELECT department_id,title,description FROM activities WHERE id=$1 FOR UPDATE")
  .bind(id).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 let dept:Uuid=row.get("department_id");
 guard_dept(&s,&c,dept,"manage").await?;
 let old_title:String=row.get("title");let old_description:String=row.get("description");
 let new_title=p.title.trim();
 if old_title!=new_title || old_description!=p.description {
  sqlx::query("UPDATE activities SET title=$2,description=$3,updated_at=now() WHERE id=$1")
   .bind(id).bind(new_title).bind(&p.description).execute(&mut *tx).await.map_err(db)?;
  sqlx::query("INSERT INTO activity_field_changes(activity_id,changed_by,old_title,new_title,old_description,new_description) VALUES($1,$2,$3,$4,$5,$6)")
   .bind(id).bind(c.sub).bind(old_title).bind(new_title).bind(old_description).bind(&p.description).execute(&mut *tx).await.map_err(db)?;
 }
 tx.commit().await.map_err(db)?;
 audit(&s,&c,"activity.rename",Some(id)).await;
 Ok(Json(json!({"ok":true})))
}
async fn activity_field_history(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1")
 .bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 guard_dept(&s,&c,dept,"read").await?;
 let rows=sqlx::query("SELECT h.id,h.old_title,h.new_title,h.old_description,h.new_description,h.changed_at,u.full_name AS author FROM activity_field_changes h JOIN users u ON u.id=h.changed_by WHERE h.activity_id=$1 ORDER BY h.changed_at DESC,h.id DESC LIMIT 100")
 .bind(id).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({
  "id":r.get::<i64,_>("id"),"old_title":r.get::<String,_>("old_title"),"new_title":r.get::<String,_>("new_title"),
  "old_description":r.get::<String,_>("old_description"),"new_description":r.get::<String,_>("new_description"),
  "changed_at":r.get::<chrono::DateTime<Utc>,_>("changed_at"),"author":r.get::<String,_>("author")
 })).collect())))
}

#[derive(Deserialize)] struct PeriodFilter {department_id:Uuid,year:i32,month:i32,kind:String}
fn check_period(y:i32,m:i32,k:&str)->ResultApi<()> { if !(2000..=2100).contains(&y)||!(1..=12).contains(&m)||!["plan","report"].contains(&k) {Err(bad("Nevaljan mjesec, godina ili vrsta dokumenta"))} else {Ok(())} }
async fn updates(State(s):State<App>,h:HeaderMap,Query(p):Query<PeriodFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"read").await?; check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT u.id,u.activity_id,u.content,u.progress,u.include_in_document,u.created_at,u.updated_at,u.creation_time_estimated,u.created_by,u.updated_by,uc.full_name AS created_by_name,uu.full_name AS updated_by_name FROM activity_updates u JOIN activities a ON a.id=u.activity_id LEFT JOIN users uc ON uc.id=u.created_by LEFT JOIN users uu ON uu.id=u.updated_by WHERE a.department_id=$1 AND u.year=$2 AND u.month=$3 AND u.kind=$4 ORDER BY a.sort_order,a.created_at").bind(p.department_id).bind(p.year).bind(p.month).bind(p.kind).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"activity_id":r.get::<Uuid,_>("activity_id"),"content":r.get::<String,_>("content"),"progress":r.get::<String,_>("progress"),"include_in_document":r.get::<bool,_>("include_in_document"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at"),"created_by":r.get::<Option<Uuid>,_>("created_by"),"updated_by":r.get::<Option<Uuid>,_>("updated_by"),"created_by_name":r.get::<Option<String>,_>("created_by_name"),"updated_by_name":r.get::<Option<String>,_>("updated_by_name"),"creation_time_estimated":r.get::<bool,_>("creation_time_estimated")})).collect())))
}
#[derive(Deserialize)] struct SaveUpdate {activity_id:Uuid,year:i32,month:i32,kind:String,content:String,progress:String,include_in_document:bool}
async fn save_update(State(s):State<App>,h:HeaderMap,Json(p):Json<SaveUpdate>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; check_period(p.year,p.month,&p.kind)?;
 let r=sqlx::query("SELECT department_id,created_by FROM activities WHERE id=$1").bind(p.activity_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 let d:Uuid=r.get("department_id"); let owner:Option<Uuid>=r.get("created_by");
 guard_dept(&s,&c,d,"write").await?;if c.role=="employee"&&owner!=Some(c.sub){return Err(forbidden());}
 if !["in_progress","completed","postponed","not_started"].contains(&p.progress.as_str()) {return Err(bad("Nevaljan status realizacije"));}
 let status:Option<String>=sqlx::query_scalar("SELECT status FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind=$4").bind(d).bind(p.year).bind(p.month).bind(&p.kind).fetch_optional(&s.db).await.map_err(db)?;
 if status.as_deref()==Some("submitted") {return Err(bad("Dokument je predan; voditelj ga prvo mora vratiti u nacrt"));}
 let id:Uuid=sqlx::query_scalar("INSERT INTO activity_updates(activity_id,year,month,kind,content,progress,include_in_document,updated_by,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$8) ON CONFLICT(activity_id,year,month,kind) DO UPDATE SET content=EXCLUDED.content, progress=EXCLUDED.progress, include_in_document=EXCLUDED.include_in_document,updated_by=EXCLUDED.updated_by,updated_at=now() RETURNING id").bind(p.activity_id).bind(p.year).bind(p.month).bind(p.kind).bind(p.content).bind(p.progress).bind(p.include_in_document).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"update.save",Some(id)).await; Ok(Json(json!({"id":id})))
}

// Carry an unfinished recurring activity into the next month without copying an old
// accomplishment into the new month's actual results.

#[derive(Deserialize)] struct CarryRequest {department_id:Uuid,year:i32,month:i32,kind:String,activity_ids:Vec<Uuid>}
#[derive(Deserialize)] struct CarryFilter {department_id:Uuid,year:i32,month:i32}
fn previous_period(year:i32,month:i32)->(i32,i32) {if month==1 {(year-1,12)} else {(year,month-1)}}
async fn carry_candidates(State(s):State<App>,h:HeaderMap,Query(p):Query<CarryFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;guard_dept(&s,&c,p.department_id,"read").await?;
 check_period(p.year,p.month,"plan")?;
 let (previous_year,previous_month)=previous_period(p.year,p.month);
 let rows=sqlx::query(r#"
 SELECT a.id,a.title,u.progress,COALESCE(u.content,'') AS previous_report,
 EXISTS(SELECT 1 FROM activity_updates next
   WHERE next.activity_id=a.id AND next.year=$2 AND next.month=$3 AND next.kind='plan') AS already_planned
 FROM activity_updates u JOIN activities a ON a.id=u.activity_id
 WHERE a.department_id=$1 AND u.year=$4 AND u.month=$5 AND u.kind='report'
 AND u.progress IN ('in_progress','postponed','not_started')
 AND a.status NOT IN ('completed','cancelled')
 ORDER BY a.sort_order,a.created_at
 "#).bind(p.department_id).bind(p.year).bind(p.month).bind(previous_year).bind(previous_month).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(json!({"source_year":previous_year,"source_month":previous_month,"items":rows.iter().map(|r|json!({
  "id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"progress":r.get::<String,_>("progress"),
  "previous_report":r.get::<String,_>("previous_report"),"already_planned":r.get::<bool,_>("already_planned")
 })).collect::<Vec<_>>()})))
}
async fn carry_forward(State(s):State<App>,h:HeaderMap,Json(p):Json<CarryRequest>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;guard_dept(&s,&c,p.department_id,"manage").await?;
 check_period(p.year,p.month,&p.kind)?;
 if p.kind!="plan" {return Err(bad("Nedovršene stavke prenose se u plan sljedećeg mjeseca"));}
 if p.activity_ids.len()>1000 {return Err(bad("Previše odabranih aktivnosti"));}
 let wanted:HashSet<Uuid>=p.activity_ids.into_iter().collect();
 let status:Option<String>=sqlx::query_scalar("SELECT status FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind='plan'")
 .bind(p.department_id).bind(p.year).bind(p.month).fetch_optional(&s.db).await.map_err(db)?;
 if status.as_deref()==Some("submitted") {return Err(bad("Predani plan prvo vratite u nacrt"));}
 let (prev_y,prev_m)=previous_period(p.year,p.month);
 let mut tx=s.db.begin().await.map_err(db)?;
 let rows=sqlx::query(r#"
 SELECT a.id FROM activity_updates u JOIN activities a ON a.id=u.activity_id
 WHERE a.department_id=$1 AND u.year=$2 AND u.month=$3 AND u.kind='report'
 AND u.progress IN ('in_progress','postponed','not_started') AND a.status NOT IN ('completed','cancelled')
 "#).bind(p.department_id).bind(prev_y).bind(prev_m).fetch_all(&mut *tx).await.map_err(db)?;
 let available:HashSet<Uuid>=rows.iter().map(|r|r.get::<Uuid,_>("id")).collect();
 if !wanted.is_subset(&available) {return Err(bad("Odabrane aktivnosti nisu nedovršene stavke prethodnog izvješća"));}
 let mut created=0u64;
 for id in wanted {
  created+=sqlx::query("INSERT INTO activity_updates(activity_id,year,month,kind,content,progress,include_in_document,updated_by,created_by) VALUES($1,$2,$3,'plan','', 'not_started',true,$4,$4) ON CONFLICT(activity_id,year,month,kind) DO NOTHING")
   .bind(id).bind(p.year).bind(p.month).bind(c.sub).execute(&mut *tx).await.map_err(db)?.rows_affected();
 }
 tx.commit().await.map_err(db)?;
 audit(&s,&c,"updates.carry_forward",Some(p.department_id)).await;
 Ok(Json(json!({"created":created,"source_year":prev_y,"source_month":prev_m})))
}

// A plan is a reference for reporting, not evidence of work already completed.
async fn plan_reference(State(s):State<App>,h:HeaderMap,Query(p):Query<PeriodFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"read").await?; check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT a.id AS activity_id,a.title,COALESCE(NULLIF(u.content,''),a.description) AS planned FROM activities a JOIN activity_updates u ON u.activity_id=a.id AND u.year=$2 AND u.month=$3 AND u.kind='plan' WHERE a.department_id=$1 AND u.include_in_document=true ORDER BY a.sort_order,a.created_at")
  .bind(p.department_id).bind(p.year).bind(p.month).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"activity_id":r.get::<Uuid,_>("activity_id"),"title":r.get::<String,_>("title"),"planned":r.get::<String,_>("planned")})).collect())))
}

#[derive(Deserialize)] struct ReorderActivities {department_id:Uuid,ordered_ids:Vec<Uuid>}
async fn reorder_activities(State(s):State<App>,h:HeaderMap,Json(p):Json<ReorderActivities>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"manage").await?;
 let mut tx=s.db.begin().await.map_err(db)?;
 let rows=sqlx::query("SELECT id FROM activities WHERE department_id=$1 FOR UPDATE")
  .bind(p.department_id).fetch_all(&mut *tx).await.map_err(db)?;
 let existing:HashSet<Uuid>=rows.iter().map(|r|r.get::<Uuid,_>("id")).collect();
 let wanted:HashSet<Uuid>=p.ordered_ids.iter().copied().collect();
 if p.ordered_ids.len()!=existing.len() || wanted!=existing {return Err(bad("Redoslijed mora sadržavati sve aktivnosti odjela, bez ponavljanja"));}
 for (i,id) in p.ordered_ids.iter().enumerate() {
  sqlx::query("UPDATE activities SET sort_order=$2,updated_at=now() WHERE id=$1")
   .bind(id).bind((i as i32)*10).execute(&mut *tx).await.map_err(db)?;
 }
 tx.commit().await.map_err(db)?;
 audit(&s,&c,"activities.reorder",Some(p.department_id)).await;
 Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)] struct MoveActivity {parent_id:Option<Uuid>}
async fn move_activity(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<MoveActivity>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 guard_dept(&s,&c,dept,"manage").await?;
 if let Some(parent)=p.parent_id {
  let pd:Option<Uuid>=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(parent).fetch_optional(&s.db).await.map_err(db)?;
  if pd!=Some(dept) {return Err(bad("Nadređena aktivnost mora biti u istom odjelu"));}
  // Guard against moving an activity below itself or one of its descendants.
  let cycle:bool=sqlx::query_scalar("WITH RECURSIVE ancestors(id,parent_id) AS (SELECT id,parent_id FROM activities WHERE id=$1 UNION ALL SELECT a.id,a.parent_id FROM activities a JOIN ancestors x ON a.id=x.parent_id) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=$2)")
   .bind(parent).bind(id).fetch_one(&s.db).await.map_err(db)?;
  if cycle {return Err(bad("Aktivnost ne može biti vlastita podstavka"));}
 }
 sqlx::query("UPDATE activities SET parent_id=$2,updated_at=now() WHERE id=$1").bind(id).bind(p.parent_id).execute(&s.db).await.map_err(db)?;
 audit(&s,&c,"activity.move",Some(id)).await;
 Ok(Json(json!({"ok":true})))
}

async fn department_docs(State(s):State<App>,h:HeaderMap,Query(p):Query<PeriodFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"read").await?; check_period(p.year,p.month,&p.kind)?;
 let row=sqlx::query("SELECT d.id,d.status,d.submitted_at,d.created_at,d.creation_time_estimated,c.full_name AS created_by_name,p.full_name AS submitted_by_name FROM department_documents d LEFT JOIN users c ON c.id=d.created_by LEFT JOIN users p ON p.id=d.submitted_by WHERE d.department_id=$1 AND year=$2 AND month=$3 AND kind=$4").bind(p.department_id).bind(p.year).bind(p.month).bind(p.kind).fetch_optional(&s.db).await.map_err(db)?;
 Ok(Json(match row {Some(r)=>json!({"id":r.get::<Uuid,_>("id"),"status":r.get::<String,_>("status"),"submitted_at":r.get::<Option<chrono::DateTime<Utc>>,_>("submitted_at"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"created_by_name":r.get::<Option<String>,_>("created_by_name"),"submitted_by_name":r.get::<Option<String>,_>("submitted_by_name"),"creation_time_estimated":r.get::<bool,_>("creation_time_estimated")}),None=>json!({"status":"draft"})}))
}
#[derive(Deserialize)] struct SubmitDept {department_id:Uuid,year:i32,month:i32,kind:String,status:String}
async fn save_department_doc(State(s):State<App>,h:HeaderMap,Json(p):Json<SubmitDept>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; guard_dept(&s,&c,p.department_id,"manage").await?; check_period(p.year,p.month,&p.kind)?;
 if !["draft","submitted"].contains(&p.status.as_str()) {return Err(bad("Nevaljan status dokumenta"));}
 if p.status=="submitted" && p.kind=="report" {
  // Use the business-local calendar, independent of the server's configured timezone.
  let current:(i32,i32)=sqlx::query_as("SELECT EXTRACT(YEAR FROM now() AT TIME ZONE 'Europe/Zagreb')::int, EXTRACT(MONTH FROM now() AT TIME ZONE 'Europe/Zagreb')::int")
    .fetch_one(&s.db).await.map_err(db)?;
  if (p.year,p.month)>current {return Err(bad("Izvješće za budući mjesec može biti nacrt, ali ga još nije moguće predati"));}
 }

 let mut tx=s.db.begin().await.map_err(db)?;
 // Protect the version sequence from concurrent submissions.
 let doc=sqlx::query("INSERT INTO department_documents(department_id,year,month,kind,status,submitted_by,submitted_at,created_by) VALUES($1,$2,$3,$4,'draft',NULL,NULL,$5) ON CONFLICT(department_id,year,month,kind) DO UPDATE SET id=department_documents.id RETURNING id")
  .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).bind(c.sub).fetch_one(&mut *tx).await.map_err(db)?;
 let id:Uuid=doc.get("id");
 let old: String=sqlx::query_scalar("SELECT status FROM department_documents WHERE id=$1 FOR UPDATE").bind(id).fetch_one(&mut *tx).await.map_err(db)?;
 let mut version:Option<i32>=None;
 if p.status=="submitted" && old!="submitted" {
  let rows=sqlx::query("SELECT a.id,a.title,a.description,a.parent_id,a.type,a.status,COALESCE(u.content,'') AS content,COALESCE(u.progress,'in_progress') AS progress FROM activities a LEFT JOIN activity_updates u ON u.activity_id=a.id AND u.year=$2 AND u.month=$3 AND u.kind=$4 WHERE a.department_id=$1 AND COALESCE(u.include_in_document,true)=true ORDER BY a.sort_order,a.created_at")
   .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_all(&mut *tx).await.map_err(db)?;
  if rows.is_empty(){return Err(bad("Nema aktivnosti uključenih u dokument"));}
  let items:Vec<Value>=rows.iter().map(|r|json!({
   "id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"description":r.get::<String,_>("description"),
   "parent_id":r.get::<Option<Uuid>,_>("parent_id"),"type":r.get::<String,_>("type"),"status":r.get::<String,_>("status"),
   "content":r.get::<String,_>("content"),"progress":r.get::<String,_>("progress")
  })).collect();
  let next:i32=sqlx::query_scalar("SELECT COALESCE(MAX(version),0)+1 FROM department_document_versions WHERE department_document_id=$1")
   .bind(id).fetch_one(&mut *tx).await.map_err(db)?;
  sqlx::query("INSERT INTO department_document_versions(department_document_id,version,snapshot,submitted_by) VALUES($1,$2,$3,$4)")
   .bind(id).bind(next).bind(json!({"department_id":p.department_id,"year":p.year,"month":p.month,"kind":p.kind,"items":items})).bind(c.sub).execute(&mut *tx).await.map_err(db)?;
  version=Some(next);
 }
 if p.status=="submitted" {
  sqlx::query("UPDATE department_documents SET status='submitted',submitted_by=$2,submitted_at=now() WHERE id=$1")
   .bind(id).bind(c.sub).execute(&mut *tx).await.map_err(db)?;
 } else {
  sqlx::query("UPDATE department_documents SET status='draft',submitted_at=NULL WHERE id=$1")
   .bind(id).execute(&mut *tx).await.map_err(db)?;
 }
 tx.commit().await.map_err(db)?;
 audit(&s,&c,if p.status=="submitted" {"department.submit"} else {"department.reopen"},Some(id)).await;
 Ok(Json(json!({"id":id,"status":p.status,"new_version":version})))
}

#[derive(Deserialize)] struct VersionsFilter {department_id:Uuid,year:i32,month:i32,kind:String}
async fn department_versions(State(s):State<App>,h:HeaderMap,Query(p):Query<VersionsFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;guard_dept(&s,&c,p.department_id,"read").await?;check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT v.id,v.version,v.snapshot,v.submitted_at,u.full_name AS author FROM department_document_versions v JOIN department_documents d ON d.id=v.department_document_id JOIN users u ON u.id=v.submitted_by WHERE d.department_id=$1 AND d.year=$2 AND d.month=$3 AND d.kind=$4 ORDER BY v.version DESC")
 .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"version":r.get::<i32,_>("version"),
 "snapshot":r.get::<Value,_>("snapshot"),"submitted_at":r.get::<chrono::DateTime<Utc>,_>("submitted_at"),"author":r.get::<String,_>("author")})).collect())))
}

// Collaboration is scoped to the activity's department. Only thread members
// and department managers/directors can read messages; all message writes
// are timestamped, append-only.

async fn inbox(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 if !matches!(c.role.as_str(),"director"|"manager"|"employee"){return Err(forbidden());}
 let sector=c.sector_id.ok_or_else(forbidden)?;
 let dept=if director(&c){None}else{Some(c.department_id.ok_or_else(forbidden)?)};
 let rows=sqlx::query(r#"
 SELECT t.id,t.title,t.status,t.updated_at,a.id AS activity_id,a.title AS activity_title,
 d.name AS department_name,a.department_id,
 (SELECT COUNT(*) FROM user_notifications n WHERE n.thread_id=t.id AND n.user_id=$1 AND n.read_at IS NULL) AS unread,
 (SELECT MAX(m.created_at) FROM activity_thread_messages m WHERE m.thread_id=t.id) AS last_message_at
 FROM activity_thread_participants p
 JOIN activity_threads t ON t.id=p.thread_id
 JOIN activities a ON a.id=t.activity_id JOIN departments d ON d.id=a.department_id
 WHERE p.user_id=$1 AND d.sector_id=$2 AND ($3::uuid IS NULL OR a.department_id=$3)
 ORDER BY t.updated_at DESC LIMIT 150
 "#).bind(c.sub).bind(sector).bind(dept).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({
  "id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"status":r.get::<String,_>("status"),
  "activity_id":r.get::<Uuid,_>("activity_id"),"activity_title":r.get::<String,_>("activity_title"),
  "department_name":r.get::<String,_>("department_name"),"department_id":r.get::<Uuid,_>("department_id"),"unread":r.get::<i64,_>("unread"),
  "updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at"),
  "last_message_at":r.get::<Option<chrono::DateTime<Utc>>,_>("last_message_at")
 })).collect())))
}
async fn mark_thread_read(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 thread_access(&s,&c,id).await?;
 let member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM activity_thread_participants WHERE thread_id=$1 AND user_id=$2)")
  .bind(id).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 if !member{return Err(forbidden());}
 sqlx::query("UPDATE activity_thread_participants SET last_read_at=now() WHERE thread_id=$1 AND user_id=$2")
  .bind(id).bind(c.sub).execute(&s.db).await.map_err(db)?;
 sqlx::query("UPDATE user_notifications SET read_at=now() WHERE thread_id=$1 AND user_id=$2 AND read_at IS NULL")
  .bind(id).bind(c.sub).execute(&s.db).await.map_err(db)?;
 Ok(Json(json!({"ok":true})))
}

async fn activity_people(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 guard_dept(&s,&c,dept,"read").await?;
 let rows=sqlx::query("SELECT id,full_name,role FROM users WHERE active=true AND (department_id=$1 OR (role='director' AND sector_id=(SELECT sector_id FROM departments WHERE id=$1))) ORDER BY full_name")
 .bind(dept).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"name":r.get::<String,_>("full_name"),"role":r.get::<String,_>("role")})).collect())))
}
async fn thread_access(s:&App,c:&Claims,id:Uuid)->ResultApi<(Uuid,Uuid,bool)> {
 let row=sqlx::query("SELECT t.activity_id,a.department_id FROM activity_threads t JOIN activities a ON a.id=t.activity_id WHERE t.id=$1")
  .bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Razgovor ne postoji"))?;
 let a:Uuid=row.get("activity_id");let dept:Uuid=row.get("department_id");
 guard_dept(s,c,dept,"read").await?;
 let supervisor=(director(c))||(c.role=="manager"&&c.department_id==Some(dept));
 let member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM activity_thread_participants WHERE thread_id=$1 AND user_id=$2)")
  .bind(id).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 if !supervisor&&!member {return Err(forbidden());}
 Ok((a,dept,supervisor))
}
async fn threads_list(State(s):State<App>,h:HeaderMap,Path(activity_id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1")
  .bind(activity_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 guard_dept(&s,&c,dept,"read").await?;
 let supervisor=director(&c)||(c.role=="manager"&&c.department_id==Some(dept));
 let rows=sqlx::query("SELECT t.id,t.title,t.status,t.created_at,t.updated_at,u.full_name AS created_name, (SELECT COUNT(*) FROM activity_thread_messages m WHERE m.thread_id=t.id) AS message_count FROM activity_threads t JOIN users u ON u.id=t.created_by WHERE t.activity_id=$1 AND ($2 OR EXISTS(SELECT 1 FROM activity_thread_participants p WHERE p.thread_id=t.id AND p.user_id=$3)) ORDER BY t.updated_at DESC")
  .bind(activity_id).bind(supervisor).bind(c.sub).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"status":r.get::<String,_>("status"),"created_name":r.get::<String,_>("created_name"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at"),"message_count":r.get::<i64,_>("message_count")})).collect())))
}
#[derive(Deserialize)] struct StartThread {title:String,participants:Vec<Uuid>,message:Option<String>}
async fn thread_create(State(s):State<App>,h:HeaderMap,Path(activity_id):Path<Uuid>,Json(p):Json<StartThread>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(activity_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 guard_dept(&s,&c,dept,"manage").await?;
 if p.title.trim().is_empty()||p.title.len()>250||p.participants.len()>30||p.message.as_ref().is_some_and(|m|m.len()>12000){return Err(bad("Nevaljan naziv, broj sudionika ili duljina poruke"));}
 let mut participants:HashSet<Uuid>=p.participants.into_iter().collect();participants.insert(c.sub);
 let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE active=true AND id = ANY($1) AND (department_id=$2 OR (role='director' AND sector_id=(SELECT sector_id FROM departments WHERE id=$2)))")
  .bind(participants.iter().copied().collect::<Vec<_>>()).bind(dept).fetch_one(&s.db).await.map_err(db)?;
 if count!=participants.len() as i64{return Err(bad("Odabrani korisnik nema pristup ovom odjelu"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 let id:Uuid=sqlx::query_scalar("INSERT INTO activity_threads(activity_id,title,created_by) VALUES($1,$2,$3) RETURNING id").bind(activity_id).bind(p.title.trim()).bind(c.sub).fetch_one(&mut *tx).await.map_err(db)?;
 for person in participants {
  sqlx::query("INSERT INTO activity_thread_participants(thread_id,user_id,added_by) VALUES($1,$2,$3)")
   .bind(id).bind(person).bind(c.sub).execute(&mut *tx).await.map_err(db)?;
  if person!=c.sub {
   sqlx::query("INSERT INTO user_notifications(user_id,thread_id,kind) VALUES($1,$2,'invited')")
    .bind(person).bind(id).execute(&mut *tx).await.map_err(db)?;
  }
 }
 if let Some(message)=p.message.filter(|m|!m.trim().is_empty()){
  sqlx::query("INSERT INTO activity_thread_messages(thread_id,author_id,body) VALUES($1,$2,$3)")
   .bind(id).bind(c.sub).bind(message.trim()).execute(&mut *tx).await.map_err(db)?;
 }
 sqlx::query("INSERT INTO activity_thread_events(thread_id,actor_id,event_type) VALUES($1,$2,'created')").bind(id).bind(c.sub).execute(&mut *tx).await.map_err(db)?;
 tx.commit().await.map_err(db)?;audit(&s,&c,"thread.create",Some(id)).await;
 Ok(Json(json!({"id":id})))
}
async fn thread_detail(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;thread_access(&s,&c,id).await?;
 let t=sqlx::query("SELECT id,title,status,created_at,updated_at FROM activity_threads WHERE id=$1").bind(id).fetch_one(&s.db).await.map_err(db)?;
 let people=sqlx::query("SELECT p.user_id,u.full_name,p.added_at FROM activity_thread_participants p JOIN users u ON u.id=p.user_id WHERE p.thread_id=$1 ORDER BY u.full_name").bind(id).fetch_all(&s.db).await.map_err(db)?;
 let messages=sqlx::query("SELECT m.id,m.body,m.created_at,m.author_id,u.full_name AS author FROM activity_thread_messages m JOIN users u ON u.id=m.author_id WHERE m.thread_id=$1 ORDER BY m.created_at,m.id LIMIT 500").bind(id).fetch_all(&s.db).await.map_err(db)?;
 let events=sqlx::query("SELECT e.event_type,e.payload,e.created_at,u.full_name AS author FROM activity_thread_events e JOIN users u ON u.id=e.actor_id WHERE e.thread_id=$1 ORDER BY e.created_at DESC LIMIT 100").bind(id).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(json!({"id":t.get::<Uuid,_>("id"),"title":t.get::<String,_>("title"),"status":t.get::<String,_>("status"),
 "created_at":t.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":t.get::<chrono::DateTime<Utc>,_>("updated_at"),
 "participants":people.iter().map(|r|json!({"id":r.get::<Uuid,_>("user_id"),"name":r.get::<String,_>("full_name"),"added_at":r.get::<chrono::DateTime<Utc>,_>("added_at")})).collect::<Vec<_>>(),
 "messages":messages.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"body":r.get::<String,_>("body"),"author":r.get::<String,_>("author"),"author_id":r.get::<Uuid,_>("author_id"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at")})).collect::<Vec<_>>(),
 "events":events.iter().map(|r|json!({"event_type":r.get::<String,_>("event_type"),"payload":r.get::<Value,_>("payload"),"author":r.get::<String,_>("author"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at")})).collect::<Vec<_>>()
 })))
}
#[derive(Deserialize)] struct SendMessage {body:String}
async fn thread_send(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<SendMessage>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;thread_access(&s,&c,id).await?;
 if p.body.trim().is_empty()||p.body.len()>12000{return Err(bad("Poruka mora sadržavati između 1 i 12000 znakova"));}
 let status:String=sqlx::query_scalar("SELECT status FROM activity_threads WHERE id=$1").bind(id).fetch_one(&s.db).await.map_err(db)?;
 if status=="done"{return Err(bad("Završeni razgovor prvo ponovno otvori"));}
 let message_id:Uuid=sqlx::query_scalar("INSERT INTO activity_thread_messages(thread_id,author_id,body) VALUES($1,$2,$3) RETURNING id").bind(id).bind(c.sub).bind(p.body.trim()).fetch_one(&s.db).await.map_err(db)?;
 sqlx::query("UPDATE activity_threads SET updated_at=now() WHERE id=$1").bind(id).execute(&s.db).await.map_err(db)?;
 sqlx::query("INSERT INTO user_notifications(user_id,thread_id,kind) SELECT user_id,$1,'message' FROM activity_thread_participants WHERE thread_id=$1 AND user_id<>$2")
  .bind(id).bind(c.sub).execute(&s.db).await.map_err(db)?;
 audit(&s,&c,"thread.message",Some(id)).await;
 Ok(Json(json!({"id":message_id})))
}
#[derive(Deserialize)] struct ThreadParticipant {user_id:Uuid}
async fn thread_add_person(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<ThreadParticipant>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;let (_,dept,supervisor)=thread_access(&s,&c,id).await?;if !supervisor{return Err(forbidden());}
 let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1 AND active=true AND (department_id=$2 OR (role='director' AND sector_id=(SELECT sector_id FROM departments WHERE id=$2))))")
 .bind(p.user_id).bind(dept).fetch_one(&s.db).await.map_err(db)?;
 if !allowed {return Err(bad("Korisnik nema pristup ovom odjelu"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 let inserted=sqlx::query("INSERT INTO activity_thread_participants(thread_id,user_id,added_by) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
 .bind(id).bind(p.user_id).bind(c.sub).execute(&mut *tx).await.map_err(db)?.rows_affected();
 if inserted>0 {
  sqlx::query("INSERT INTO activity_thread_events(thread_id,actor_id,event_type,payload) VALUES($1,$2,'participant_added',$3)")
   .bind(id).bind(c.sub).bind(json!({"user_id":p.user_id})).execute(&mut *tx).await.map_err(db)?;
  sqlx::query("UPDATE activity_threads SET updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(db)?;
  sqlx::query("INSERT INTO user_notifications(user_id,thread_id,kind) VALUES($1,$2,'invited')")
   .bind(p.user_id).bind(id).execute(&mut *tx).await.map_err(db)?;
 }
 tx.commit().await.map_err(db)?;Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)] struct ThreadStatus {status:String}
async fn thread_status(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<ThreadStatus>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;let (_,_,supervisor)=thread_access(&s,&c,id).await?;if !supervisor{return Err(forbidden());}
 if !["open","done"].contains(&p.status.as_str()){return Err(bad("Nevaljan status"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 sqlx::query("UPDATE activity_threads SET status=$2,updated_at=now() WHERE id=$1").bind(id).bind(&p.status).execute(&mut *tx).await.map_err(db)?;
 sqlx::query("INSERT INTO activity_thread_events(thread_id,actor_id,event_type,payload) VALUES($1,$2,'status_changed',$3)").bind(id).bind(c.sub).bind(json!({"status":p.status})).execute(&mut *tx).await.map_err(db)?;
 tx.commit().await.map_err(db)?;audit(&s,&c,"thread.status",Some(id)).await;Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)] struct SectorFreshnessFilter {year:i32,month:i32,kind:String}
async fn sector_freshness(State(s):State<App>,h:HeaderMap,Query(p):Query<SectorFreshnessFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?;if !director(&c){return Err(forbidden());}check_period(p.year,p.month,&p.kind)?;
 let sector=c.sector_id.ok_or_else(forbidden)?;
 let rows=sqlx::query("SELECT dd.department_id, COALESCE(MAX(v.version),0) AS version FROM department_documents dd JOIN departments d ON d.id=dd.department_id LEFT JOIN department_document_versions v ON v.department_document_id=dd.id WHERE dd.year=$1 AND dd.month=$2 AND dd.kind=$3 AND d.sector_id=$4 GROUP BY dd.department_id")
 .bind(p.year).bind(p.month).bind(&p.kind).bind(sector).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"department_id":r.get::<Uuid,_>("department_id"),"version":r.get::<i64,_>("version")})).collect())))
}
#[derive(Deserialize)] struct SectorFilter {year:i32,month:i32,kind:String}
async fn sector_docs(State(s):State<App>,h:HeaderMap,Query(p):Query<SectorFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; if !director(&c) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT d.id,d.title,d.status,d.revision,d.created_at,d.snapshot,d.introduction,d.conclusion,u.full_name AS created_by_name FROM sector_documents d LEFT JOIN users u ON u.id=d.created_by WHERE d.year=$1 AND month=$2 AND kind=$3 AND d.sector_id=$4 ORDER BY d.revision DESC").bind(p.year).bind(p.month).bind(p.kind).bind(c.sector_id.ok_or_else(forbidden)?).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"status":r.get::<String,_>("status"),"revision":r.get::<i32,_>("revision"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"created_by_name":r.get::<Option<String>,_>("created_by_name"),"snapshot":r.get::<Value,_>("snapshot"),"introduction":r.get::<String,_>("introduction"),"conclusion":r.get::<String,_>("conclusion")})).collect())))
}
#[derive(Deserialize)] struct BuildSector {year:i32,month:i32,kind:String,department_ids:Vec<Uuid>,introduction:Option<String>,conclusion:Option<String>}
async fn build_sector(State(s):State<App>,h:HeaderMap,Json(p):Json<BuildSector>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; if !director(&c) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 if p.department_ids.is_empty() {return Err(bad("Odaberite barem jedan odjel"));}
 let mut sections=Vec::new();
 for dept_id in &p.department_ids {
  let d=sqlx::query("SELECT name,code FROM departments WHERE id=$1 AND sector_id=$2").bind(dept_id).bind(c.sector_id.ok_or_else(forbidden)?).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Nepoznat odjel"))?;
  let accepted:Option<String>=sqlx::query_scalar("SELECT status FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind=$4").bind(dept_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_optional(&s.db).await.map_err(db)?;
  if accepted.as_deref()!=Some("submitted") {return Err(bad(format!("Odjel {} još nije predao dokument",d.get::<String,_>("name"))));}
  // A director must consume the submitted immutable snapshot, NOT live activities.
  // This prevents subsequent edits from silently changing an accepted report.
  let version_row=sqlx::query("SELECT v.version,v.snapshot,v.submitted_at,u.full_name AS submitted_by_name FROM department_document_versions v JOIN users u ON u.id=v.submitted_by JOIN department_documents dd ON dd.id=v.department_document_id WHERE dd.department_id=$1 AND dd.year=$2 AND dd.month=$3 AND dd.kind=$4 ORDER BY v.version DESC LIMIT 1")
   .bind(dept_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Odjel nema spremljenu predanu verziju"))?;
  let source_version:i32=version_row.get("version");
  let immutable:Value=version_row.get("snapshot");
  let source_submitted_at:chrono::DateTime<Utc>=version_row.get("submitted_at");
  let source_submitted_by:String=version_row.get("submitted_by_name");
  let items=immutable.get("items").and_then(Value::as_array).ok_or_else(||bad("Predana verzija nema popis aktivnosti"))?;
  if items.is_empty(){return Err(bad("Predana verzija odjela nema aktivnosti"));}
  sections.push(json!({"department_id":dept_id,"code":d.get::<String,_>("code"),"name":d.get::<String,_>("name"),"source_version":source_version,"source_submitted_at":source_submitted_at,"source_submitted_by":source_submitted_by,"items":items}));
 }
 let title=format!("{} značajnijih aktivnosti za {:02}.{}.",if p.kind=="plan" {"PLAN"}else{"IZVJEŠĆE"},p.month,p.year);
 let snapshot=json!({"year":p.year,"month":p.month,"kind":p.kind,"title":title,"sections":sections});
 let revision:i32=sqlx::query_scalar("SELECT COALESCE(MAX(revision),0)+1 FROM sector_documents WHERE year=$1 AND month=$2 AND kind=$3 AND sector_id=$4").bind(p.year).bind(p.month).bind(&p.kind).bind(c.sector_id.ok_or_else(forbidden)?).fetch_one(&s.db).await.map_err(db)?;
 let id:Uuid=sqlx::query_scalar("INSERT INTO sector_documents(year,month,kind,title,introduction,conclusion,snapshot,revision,created_by,sector_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id").bind(p.year).bind(p.month).bind(&p.kind).bind(&title).bind(p.introduction.unwrap_or_default()).bind(p.conclusion.unwrap_or_default()).bind(snapshot).bind(revision).bind(c.sub).bind(c.sector_id.ok_or_else(forbidden)?).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"sector.build",Some(id)).await; Ok(Json(json!({"id":id,"revision":revision})))
}

// Editing a sector draft changes ONLY its snapshot, never the underlying activities
// or submitted department reports. Locked revisions stay immutable.
#[derive(Deserialize)] struct EditSector {snapshot:Value,introduction:String,conclusion:String}
async fn edit_sector(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<EditSector>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; if !director(&c) {return Err(forbidden());}
 let row=sqlx::query("SELECT snapshot,status FROM sector_documents WHERE id=$1 AND sector_id=$2").bind(id).bind(c.sector_id.ok_or_else(forbidden)?).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Dokument nije pronađen"))?;
 if row.get::<String,_>("status")!="draft" {return Err(bad("Zaključana revizija se ne može mijenjati"));}
 let old:Value=row.get("snapshot");
 let original=old.get("sections").and_then(Value::as_array).ok_or_else(||bad("Neispravan izvorni dokument"))?;
 let sections=p.snapshot.get("sections").and_then(Value::as_array).ok_or_else(||bad("Nedostaju poglavlja"))?;
 if sections.len()!=original.len() || p.introduction.len()>20000 || p.conclusion.len()>20000 {return Err(bad("Neispravna struktura ili predugačak tekst"));}
 let mut original_ids=HashSet::new(); let mut original_sections=HashSet::new();
 for sec in original {
  let sid=sec.get("department_id").and_then(Value::as_str).ok_or_else(||bad("Nedostaje odjel"))?;
  original_sections.insert(sid.to_string());
  for item in sec.get("items").and_then(Value::as_array).ok_or_else(||bad("Nedostaju stavke"))? {
   let itemid=item.get("id").and_then(Value::as_str).ok_or_else(||bad("Nedostaje ID stavke"))?;
   original_ids.insert((sid.to_string(),itemid.to_string()));
  }
 }
 let mut seen_sections=HashSet::new(); let mut seen_items=HashSet::new(); let mut new_sections=Vec::new();
 for sec in sections {
  let sid=sec.get("department_id").and_then(Value::as_str).ok_or_else(||bad("Nedostaje odjel"))?;
  if !original_sections.contains(sid) || !seen_sections.insert(sid.to_string()) {return Err(bad("Nevaljan odjel"));}
  let source=original.iter().find(|s|s.get("department_id").and_then(Value::as_str)==Some(sid)).unwrap();
  let items=sec.get("items").and_then(Value::as_array).ok_or_else(||bad("Nedostaju stavke"))?;
  if items.len()>1000 {return Err(bad("Previše stavki"));}
  let mut cleaned=Vec::new();
  for item in items {
   let itemid=item.get("id").and_then(Value::as_str).ok_or_else(||bad("Nedostaje ID stavke"))?;
   if !original_ids.contains(&(sid.to_string(),itemid.to_string())) || !seen_items.insert(itemid.to_string()) {return Err(bad("Nepoznata ili ponovljena stavka"));}
   let base=source.get("items").and_then(Value::as_array).unwrap().iter().find(|i|i.get("id").and_then(Value::as_str)==Some(itemid)).unwrap();
   let title=item.get("title").and_then(Value::as_str).ok_or_else(||bad("Nedostaje naslov"))?;
   let description=item.get("description").and_then(Value::as_str).unwrap_or("");
   let content=item.get("content").and_then(Value::as_str).unwrap_or("");
   if title.trim().is_empty() || title.len()>1000 || description.len()>12000 || content.len()>12000 {return Err(bad("Neispravan naslov ili predugačak opis"));}
   let mut copy=base.clone(); copy["title"]=json!(title);copy["description"]=json!(description);copy["content"]=json!(content);
   cleaned.push(copy);
  }
  let mut copy=source.clone();copy["items"]=json!(cleaned);new_sections.push(copy);
 }
 if seen_items.is_empty() {return Err(bad("Sektorski dokument mora sadržavati barem jednu stavku"));}
 if new_sections.iter().all(|s| s.get("items").and_then(Value::as_array).map(|a|a.is_empty()).unwrap_or(true)) {return Err(bad("Završni dokument mora sadržavati barem jednu stavku"));}
 let mut snapshot=old; snapshot["sections"]=json!(new_sections);
 let mut tx=s.db.begin().await.map_err(db)?;
 let n=sqlx::query("UPDATE sector_documents SET snapshot=$2,introduction=$3,conclusion=$4 WHERE id=$1 AND status='draft' AND sector_id=$5")
  .bind(id).bind(&snapshot).bind(&p.introduction).bind(&p.conclusion).bind(c.sector_id.ok_or_else(forbidden)?).execute(&mut *tx).await.map_err(db)?.rows_affected();
 if n==0 {return Err(bad("Dokument je u međuvremenu zaključan"));}
 sqlx::query("INSERT INTO sector_document_edits(sector_document_id,snapshot,introduction,conclusion,edited_by) VALUES($1,$2,$3,$4,$5)")
  .bind(id).bind(snapshot).bind(&p.introduction).bind(&p.conclusion).bind(c.sub).execute(&mut *tx).await.map_err(db)?;
 tx.commit().await.map_err(db)?;
 audit(&s,&c,"sector.edit",Some(id)).await;Ok(Json(json!({"ok":true})))
}

async fn lock_sector(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h).await?; if !director(&c) {return Err(forbidden());}
 let n=sqlx::query("UPDATE sector_documents SET status='locked',locked_at=now() WHERE id=$1 AND status='draft' AND sector_id=$2").bind(id).bind(c.sector_id.ok_or_else(forbidden)?).execute(&s.db).await.map_err(db)?.rows_affected();
 if n==0 {return Err(bad("Dokument nije nacrt ili ne postoji"));} audit(&s,&c,"sector.lock",Some(id)).await;
 Ok(Json(json!({"ok":true})))
}
async fn export_doc(State(s):State<App>,h:HeaderMap,Path((id,format)):Path<(Uuid,String)>)->ResultApi<Response> {
 let c=who(&s,&h).await?; if !director(&c) {return Err(forbidden());}
 if !["docx","pdf","modern-pdf","modern-letterhead-pdf"].contains(&format.as_str()) {return Err(bad("Nepodržani format"));}
 let r=sqlx::query("SELECT title,introduction,conclusion,snapshot,revision FROM sector_documents WHERE id=$1 AND sector_id=$2").bind(id).bind(c.sector_id.ok_or_else(forbidden)?).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Nepostojeći dokument"))?;
 let payload=json!({"title":r.get::<String,_>("title"),"introduction":r.get::<String,_>("introduction"),"conclusion":r.get::<String,_>("conclusion"),"snapshot":r.get::<Value,_>("snapshot")});
 let resp=s.client.post(format!("{}/render/{}",s.doc_url,format)).json(&payload).send().await.map_err(|e|ErrApi(StatusCode::BAD_GATEWAY,e.to_string()))?;
 if !resp.status().is_success() {return Err(ErrApi(StatusCode::BAD_GATEWAY,resp.text().await.unwrap_or_default()));}
 let bytes=resp.bytes().await.map_err(|e|ErrApi(StatusCode::BAD_GATEWAY,e.to_string()))?;
 let mime=if format=="pdf"||format=="modern-pdf"||format=="modern-letterhead-pdf" {"application/pdf"} else {"application/vnd.openxmlformats-officedocument.wordprocessingml.document"};
 Ok(Response::builder().status(StatusCode::OK).header(header::CONTENT_TYPE,mime).header(header::CONTENT_DISPOSITION,format!("attachment; filename=reportflow-{}{}.{}",id,if format=="modern-pdf" {"-modern"}else if format=="modern-letterhead-pdf" {"-modern-memorandum"}else{""},if format=="docx" {"docx"}else{"pdf"})).body(Body::from(bytes)).unwrap())
}
async fn health()->Json<Value> {Json(json!({"status":"ok"}))}
#[tokio::main] async fn main()->anyhow::Result<()> {
 tracing_subscriber::fmt::init();
 let url=env::var("DATABASE_URL")?;
 let secret=env::var("JWT_SECRET")?;
 if secret.len()<32 { anyhow::bail!("JWT_SECRET mora imati najmanje 32 znaka"); }
 let db=PgPoolOptions::new().max_connections(10).connect(&url).await?;
 sqlx::migrate!("./migrations").run(&db).await?;
 if let (Ok(user),Ok(pass))=(env::var("BOOTSTRAP_ADMIN"),env::var("BOOTSTRAP_PASSWORD")) {
  if pass.len()<10 { anyhow::bail!("BOOTSTRAP_PASSWORD mora imati najmanje 10 znakova"); }
  let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username=$1)").bind(&user).fetch_one(&db).await?;
  if !exists { sqlx::query("INSERT INTO users(username,full_name,password_hash,role) VALUES($1,'Administrator',$2,'admin')").bind(user).bind(hash(&pass).map_err(|e|anyhow::anyhow!(e.1))?).execute(&db).await?; }
 }
 let app=App{db,secret:Arc::new(secret),doc_url:env::var("DOC_SERVICE_URL").unwrap_or("http://documents:8081".into()),client:reqwest::Client::new()};
 let routes=Router::new()
  .route("/api/health",get(health)).route("/api/auth/login",post(login)).route("/api/auth/me",get(me))
  .route("/api/sectors",get(sectors).post(create_sector))
  .route("/api/departments",get(departments).post(create_department))
  .route("/api/users",get(users).post(create_user)).route("/api/users/{id}",patch(update_user))
  .route("/api/dashboard",get(dashboard))
  .route("/api/activities",get(activities).post(create_activity)).route("/api/activities/{id}",patch(edit_activity))
  .route("/api/activity-search",get(search_activities))
  .route("/api/activities/{id}/history",get(activity_history))
  .route("/api/activities/{id}/edit-details",patch(rename_activity))
  .route("/api/activities/{id}/field-history",get(activity_field_history))
  .route("/api/activities/{id}/people",get(activity_people))
  .route("/api/activities/{id}/threads",get(threads_list).post(thread_create))
  .route("/api/threads/{id}",get(thread_detail))
  .route("/api/inbox",get(inbox))
  .route("/api/threads/{id}/read",post(mark_thread_read))
  .route("/api/threads/{id}/messages",post(thread_send))
  .route("/api/threads/{id}/participants",post(thread_add_person))
  .route("/api/threads/{id}/status",patch(thread_status))
  .route("/api/updates",get(updates).post(save_update))
  .route("/api/updates/carry-forward",post(carry_forward))
  .route("/api/updates/carry-candidates",get(carry_candidates))
  .route("/api/plan-reference",get(plan_reference))
  .route("/api/activities/reorder",post(reorder_activities))
  .route("/api/activities/{id}/move",post(move_activity))
  .route("/api/department-documents",get(department_docs).post(save_department_doc))
  .route("/api/department-document-versions",get(department_versions))
  .route("/api/sector-document-freshness",get(sector_freshness))
  .route("/api/sector-documents",get(sector_docs).post(build_sector))
  .route("/api/sector-documents/{id}",patch(edit_sector))
  .route("/api/sector-documents/{id}/lock",post(lock_sector))
  .route("/api/sector-documents/{id}/export/{format}",get(export_doc))
  .with_state(app).layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)).layer(TraceLayer::new_for_http());
 let listener=tokio::net::TcpListener::bind("0.0.0.0:8095").await?;
 tracing::info!("ReportFlow API listening on 8095"); axum::serve(listener,routes).await?; Ok(())
}

#[cfg(test)]
mod history_search_tests {
 use super::search_pattern;
 #[test] fn wildcard_characters_are_literal() {
  assert_eq!(search_pattern(r"VHF_100%"),r"%VHF\_100\%%");
  assert_eq!(search_pattern(r"test\me"),r"%test\\me%");
 }
 #[test] fn ordinary_search_is_case_agnostic_at_sql_layer() {
  assert_eq!(search_pattern("Jadrija"),"%Jadrija%");
 }
}

#[cfg(test)]
mod dashboard_tests {
 use super::dashboard_period_valid;
 #[test]
 fn valid_periods() {
  assert!(dashboard_period_valid(2026,11));
  assert!(dashboard_period_valid(2000,1));
  assert!(dashboard_period_valid(2100,12));
 }
 #[test]
 fn invalid_periods() {
  for (year,month) in [(1999,11),(2101,1),(2026,0),(2026,13)] {
   assert!(!dashboard_period_valid(year,month));
  }
 }
}
