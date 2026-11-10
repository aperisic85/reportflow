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
#[derive(Debug, Serialize, Deserialize)] struct Claims { sub:Uuid, role:String, department_id:Option<Uuid>, exp:usize }
fn who(s:&App,h:&HeaderMap)->ResultApi<Claims> {
 let token=h.get(header::AUTHORIZATION).and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).ok_or_else(unauthorized)?;
 let c=decode::<Claims>(token,&DecodingKey::from_secret(s.secret.as_bytes()),&Validation::default()).map_err(|_|unauthorized())?.claims;
 Ok(c)
}
fn can_read(c:&Claims, dept:Uuid)->bool { matches!(c.role.as_str(),"admin"|"director") || c.department_id==Some(dept) }
fn can_write(c:&Claims,dept:Uuid)->bool { c.role=="admin" || (matches!(c.role.as_str(),"manager"|"employee") && c.department_id==Some(dept)) }
fn can_manage(c:&Claims,dept:Uuid)->bool { c.role=="admin" || (c.role=="manager" && c.department_id==Some(dept)) }
fn director(c:&Claims)->bool { matches!(c.role.as_str(),"admin"|"director") }
async fn audit(s:&App,c:&Claims,action:&str,id:Option<Uuid>) { let _=sqlx::query("INSERT INTO audit_events(user_id,action,entity_id) VALUES($1,$2,$3)").bind(c.sub).bind(action).bind(id).execute(&s.db).await; }
fn hash(p:&str)->ResultApi<String> { let salt=SaltString::generate(&mut OsRng); Ok(Argon2::default().hash_password(p.as_bytes(),&salt).map_err(|e|bad(e.to_string()))?.to_string()) }

#[derive(Deserialize)] struct Login { username:String,password:String }
async fn login(State(s):State<App>,Json(p):Json<Login>)->ResultApi<Json<Value>> {
 let r=sqlx::query("SELECT id,username,full_name,role,department_id,password_hash FROM users WHERE username=$1 AND active=true").bind(&p.username).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(unauthorized)?;
 let pass:String=r.get("password_hash");
 let valid=PasswordHash::new(&pass).ok().map(|ph|Argon2::default().verify_password(p.password.as_bytes(),&ph).is_ok()).unwrap_or(false);
 if !valid {return Err(unauthorized());}
 let c=Claims{sub:r.get("id"),role:r.get("role"),department_id:r.get("department_id"),exp:(Utc::now().timestamp()+60*60*12) as usize};
 let token=encode(&Header::default(),&c,&EncodingKey::from_secret(s.secret.as_bytes())).map_err(|e|bad(e.to_string()))?;
 Ok(Json(json!({"token":token,"user":{"id":c.sub,"username":p.username,"name":r.get::<String,_>("full_name"),"role":c.role,"department_id":c.department_id}})))
}
async fn me(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;
 let r=sqlx::query("SELECT username,full_name FROM users WHERE id=$1 AND active=true").bind(c.sub).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(unauthorized)?;
 Ok(Json(json!({"id":c.sub,"username":r.get::<String,_>("username"),"name":r.get::<String,_>("full_name"),"role":c.role,"department_id":c.department_id})))
}
async fn departments(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let _=who(&s,&h)?;
 let rows=sqlx::query("SELECT id,code,name,sort_order FROM departments ORDER BY sort_order").fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"code":r.get::<String,_>("code"),"name":r.get::<String,_>("name"),"sort_order":r.get::<i32,_>("sort_order")})).collect())))
}
#[derive(Deserialize)] struct NewUser { username:String,full_name:String,password:String,role:String,department_id:Option<Uuid> }
async fn create_user(State(s):State<App>,h:HeaderMap,Json(p):Json<NewUser>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if c.role!="admin" {return Err(forbidden());}
 if !["admin","director","manager","employee"].contains(&p.role.as_str()) || p.password.len()<10 {return Err(bad("Uloga nije valjana ili je lozinka kraća od 10 znakova"));}
 if matches!(p.role.as_str(),"manager"|"employee") && p.department_id.is_none() {return Err(bad("Odjel je obvezan"));}
 let id:Uuid=sqlx::query_scalar("INSERT INTO users(username,full_name,password_hash,role,department_id) VALUES($1,$2,$3,$4,$5) RETURNING id").bind(p.username).bind(p.full_name).bind(hash(&p.password)?).bind(p.role).bind(p.department_id).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"user.create",Some(id)).await; Ok(Json(json!({"id":id})))
}
async fn users(State(s):State<App>,h:HeaderMap)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if c.role!="admin" {return Err(forbidden());}
 let rows=sqlx::query("SELECT id,username,full_name,role,department_id,active FROM users ORDER BY full_name").fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"username":r.get::<String,_>("username"),"full_name":r.get::<String,_>("full_name"),"role":r.get::<String,_>("role"),"department_id":r.get::<Option<Uuid>,_>("department_id"),"active":r.get::<bool,_>("active")})).collect())))
}

#[derive(Deserialize)] struct DashboardFilter { year:i32, month:i32 }
fn dashboard_period_valid(year:i32,month:i32)->bool { (2000..=2100).contains(&year) && (1..=12).contains(&month) }
async fn dashboard(State(s):State<App>,h:HeaderMap,Query(p):Query<DashboardFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;
 if !dashboard_period_valid(p.year,p.month) {
  return Err(bad("Nevaljan mjesec ili godina"));
 }
 let scoped_dept=if director(&c) {None} else {
  Some(c.department_id.ok_or_else(forbidden)?)
 };
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
 FROM departments d WHERE ($1::uuid IS NULL OR d.id=$1)
 ORDER BY d.sort_order,d.name
 "#).bind(scoped_dept).bind(p.year).bind(p.month).fetch_all(&s.db).await.map_err(db)?;
 let recent=sqlx::query(r#"
 SELECT u.activity_id,a.title,d.name AS department_name,u.year,u.month,u.kind,
        u.content,u.progress,u.updated_at
 FROM activity_updates u JOIN activities a ON a.id=u.activity_id
 JOIN departments d ON d.id=a.department_id
 WHERE ($1::uuid IS NULL OR d.id=$1)
 ORDER BY u.updated_at DESC,u.id DESC LIMIT 8
 "#).bind(scoped_dept).fetch_all(&s.db).await.map_err(db)?;
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
 let c=who(&s,&h)?; if !can_read(&c,p.department_id) {return Err(forbidden());}
 let rows=sqlx::query("SELECT id,department_id,parent_id,title,description,type,status,sort_order,created_by,created_at,updated_at FROM activities WHERE department_id=$1 ORDER BY sort_order,created_at").bind(p.department_id).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"department_id":r.get::<Uuid,_>("department_id"),"parent_id":r.get::<Option<Uuid>,_>("parent_id"),"title":r.get::<String,_>("title"),"description":r.get::<String,_>("description"),"type":r.get::<String,_>("type"),"status":r.get::<String,_>("status"),"sort_order":r.get::<i32,_>("sort_order"),"created_by":r.get::<Option<Uuid>,_>("created_by"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at")})).collect())))
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
 let c=who(&s,&h)?;
 let term=p.q.trim();
 if term.chars().count()<2 || term.chars().count()>120 {return Err(bad("Pretraga zahtijeva 2 do 120 znakova"));}
 if let Some(y)=p.year {if !(2000..=2100).contains(&y){return Err(bad("Nevaljana godina"));}}
 if let Some(m)=p.month {if !(1..=12).contains(&m){return Err(bad("Nevaljan mjesec"));}}
 if let Some(k)=&p.kind {if !["plan","report"].contains(&k.as_str()){return Err(bad("Nevaljana vrsta dokumenta"));}}
 let scope=if director(&c) {p.department_id} else {
  let d=c.department_id.ok_or_else(forbidden)?;
  if p.department_id.is_some() && p.department_id!=Some(d) {return Err(forbidden());}
  Some(d)
 };
 let pattern=search_pattern(term);
 let rows=sqlx::query(r#"
 SELECT a.id AS activity_id,a.department_id,d.name AS department_name,
        a.title,a.description,a.type,a.status,
        u.year,u.month,u.kind,u.content,u.progress,u.include_in_document,u.updated_at
 FROM activities a
 JOIN departments d ON d.id=a.department_id
 LEFT JOIN activity_updates u ON u.activity_id=a.id
 WHERE ($1::uuid IS NULL OR a.department_id=$1)
   AND (a.title ILIKE $2 ESCAPE '\' OR a.description ILIKE $2 ESCAPE '\'
        OR u.content ILIKE $2 ESCAPE '\')
   AND ($3::int IS NULL OR u.year=$3)
   AND ($4::int IS NULL OR u.month=$4)
   AND ($5::text IS NULL OR u.kind=$5)
 ORDER BY a.title,u.year DESC NULLS LAST,u.month DESC NULLS LAST,u.kind
 LIMIT $6
 "#)
 .bind(scope).bind(pattern).bind(p.year).bind(p.month).bind(p.kind)
 .bind(p.limit.unwrap_or(50).clamp(1,100))
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
 let c=who(&s,&h)?;
 let a=sqlx::query("SELECT a.id,a.department_id,d.name AS department_name,a.parent_id,a.title,a.description,a.type,a.status FROM activities a JOIN departments d ON d.id=a.department_id WHERE a.id=$1")
  .bind(id).fetch_optional(&s.db).await.map_err(db)?
  .ok_or_else(||ErrApi(StatusCode::NOT_FOUND,"Aktivnost nije pronađena".into()))?;
 let dept:Uuid=a.get("department_id");
 if !can_read(&c,dept) {return Err(forbidden());}
 let rows=sqlx::query(r#"
 SELECT u.id,u.year,u.month,u.kind,u.content,u.progress,u.include_in_document,u.updated_at,
        usr.full_name AS updated_by_name
 FROM activity_updates u
 LEFT JOIN users usr ON usr.id=u.updated_by
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
  "updated_by_name":r.get::<Option<String>,_>("updated_by_name")
 })).collect();
 Ok(Json(json!({"activity":{
  "id":a.get::<Uuid,_>("id"),"department_id":dept,
  "department_name":a.get::<String,_>("department_name"),
  "parent_id":a.get::<Option<Uuid>,_>("parent_id"),
  "title":a.get::<String,_>("title"),"description":a.get::<String,_>("description"),
  "type":a.get::<String,_>("type"),"status":a.get::<String,_>("status")
 },"entries":entries})))
}

#[derive(Deserialize)] struct NewActivity { department_id:Uuid,parent_id:Option<Uuid>,title:String,description:Option<String>,#[serde(rename="type")] activity_type:String,sort_order:Option<i32> }
async fn create_activity(State(s):State<App>,h:HeaderMap,Json(p):Json<NewActivity>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !can_write(&c,p.department_id) {return Err(forbidden());}
 if p.title.trim().is_empty() || !["ongoing","project","one_off"].contains(&p.activity_type.as_str()) {return Err(bad("Naziv ili vrsta aktivnosti nije valjana"));}
 if let Some(parent)=p.parent_id { let d:Option<Uuid>=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(parent).fetch_optional(&s.db).await.map_err(db)?; if d!=Some(p.department_id) {return Err(bad("Podstavka mora biti iz istog odjela"));} }
 let id:Uuid=sqlx::query_scalar("INSERT INTO activities(department_id,parent_id,title,description,type,sort_order,created_by) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id").bind(p.department_id).bind(p.parent_id).bind(p.title).bind(p.description.unwrap_or_default()).bind(p.activity_type).bind(p.sort_order.unwrap_or(0)).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"activity.create",Some(id)).await; Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)] struct EditActivity {title:String,description:String,status:String,sort_order:i32}
async fn edit_activity(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<EditActivity>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;
 let row=sqlx::query("SELECT department_id,created_by FROM activities WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 let dept:Uuid=row.get("department_id"); let owner:Option<Uuid>=row.get("created_by");
 if !can_write(&c,dept) || (c.role=="employee" && owner!=Some(c.sub)) {return Err(forbidden());}
 if !["in_progress","completed","paused","cancelled"].contains(&p.status.as_str()) {return Err(bad("Nevaljan status"));}
 sqlx::query("UPDATE activities SET title=$2,description=$3,status=$4,sort_order=$5,updated_at=now() WHERE id=$1").bind(id).bind(p.title).bind(p.description).bind(p.status).bind(p.sort_order).execute(&s.db).await.map_err(db)?;
 audit(&s,&c,"activity.update",Some(id)).await; Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)] struct PeriodFilter {department_id:Uuid,year:i32,month:i32,kind:String}
fn check_period(y:i32,m:i32,k:&str)->ResultApi<()> { if !(2000..=2100).contains(&y)||!(1..=12).contains(&m)||!["plan","report"].contains(&k) {Err(bad("Nevaljan mjesec, godina ili vrsta dokumenta"))} else {Ok(())} }
async fn updates(State(s):State<App>,h:HeaderMap,Query(p):Query<PeriodFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !can_read(&c,p.department_id) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT u.id,u.activity_id,u.content,u.progress,u.include_in_document,u.updated_at FROM activity_updates u JOIN activities a ON a.id=u.activity_id WHERE a.department_id=$1 AND u.year=$2 AND u.month=$3 AND u.kind=$4 ORDER BY a.sort_order,a.created_at").bind(p.department_id).bind(p.year).bind(p.month).bind(p.kind).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"activity_id":r.get::<Uuid,_>("activity_id"),"content":r.get::<String,_>("content"),"progress":r.get::<String,_>("progress"),"include_in_document":r.get::<bool,_>("include_in_document"),"updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at")})).collect())))
}
#[derive(Deserialize)] struct SaveUpdate {activity_id:Uuid,year:i32,month:i32,kind:String,content:String,progress:String,include_in_document:bool}
async fn save_update(State(s):State<App>,h:HeaderMap,Json(p):Json<SaveUpdate>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; check_period(p.year,p.month,&p.kind)?;
 let r=sqlx::query("SELECT department_id,created_by FROM activities WHERE id=$1").bind(p.activity_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 let d:Uuid=r.get("department_id"); let owner:Option<Uuid>=r.get("created_by");
 if !can_write(&c,d) || (c.role=="employee" && owner!=Some(c.sub)) {return Err(forbidden());}
 if !["in_progress","completed","postponed","not_started"].contains(&p.progress.as_str()) {return Err(bad("Nevaljan status realizacije"));}
 let status:Option<String>=sqlx::query_scalar("SELECT status FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind=$4").bind(d).bind(p.year).bind(p.month).bind(&p.kind).fetch_optional(&s.db).await.map_err(db)?;
 if status.as_deref()==Some("submitted") {return Err(bad("Dokument je predan; voditelj ga prvo mora vratiti u nacrt"));}
 let id:Uuid=sqlx::query_scalar("INSERT INTO activity_updates(activity_id,year,month,kind,content,progress,include_in_document,updated_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(activity_id,year,month,kind) DO UPDATE SET content=EXCLUDED.content, progress=EXCLUDED.progress, include_in_document=EXCLUDED.include_in_document,updated_by=EXCLUDED.updated_by,updated_at=now() RETURNING id").bind(p.activity_id).bind(p.year).bind(p.month).bind(p.kind).bind(p.content).bind(p.progress).bind(p.include_in_document).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"update.save",Some(id)).await; Ok(Json(json!({"id":id})))
}

// Carry an unfinished recurring activity into the next month without copying an old
// accomplishment into the new month's actual results.
#[derive(Deserialize)] struct CarryRequest {department_id:Uuid,year:i32,month:i32,kind:String}
async fn carry_forward(State(s):State<App>,h:HeaderMap,Json(p):Json<CarryRequest>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !can_manage(&c,p.department_id) {return Err(forbidden());}
 check_period(p.year,p.month,&p.kind)?;
 let status:Option<String>=sqlx::query_scalar("SELECT status FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind=$4")
  .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_optional(&s.db).await.map_err(db)?;
 if status.as_deref()==Some("submitted") {return Err(bad("Predani dokument prvo vratite u nacrt"));}
 let (prev_y,prev_m)=if p.month==1 {(p.year-1,12)} else {(p.year,p.month-1)};
 let n=sqlx::query("INSERT INTO activity_updates(activity_id,year,month,kind,content,progress,include_in_document,updated_by) SELECT a.id,$2,$3,$4,'','not_started',u.include_in_document,$7 FROM activity_updates u JOIN activities a ON a.id=u.activity_id WHERE a.department_id=$1 AND u.year=$5 AND u.month=$6 AND u.kind=$4 AND a.type IN ('ongoing','project') AND a.status='in_progress' AND u.progress NOT IN ('completed') ON CONFLICT(activity_id,year,month,kind) DO NOTHING")
  .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).bind(prev_y).bind(prev_m).bind(c.sub).execute(&s.db).await.map_err(db)?.rows_affected();
 audit(&s,&c,"updates.carry_forward",Some(p.department_id)).await;
 Ok(Json(json!({"created":n,"source_year":prev_y,"source_month":prev_m})))
}

// A plan is a reference for reporting, not evidence of work already completed.
async fn plan_reference(State(s):State<App>,h:HeaderMap,Query(p):Query<PeriodFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !can_read(&c,p.department_id) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT a.id AS activity_id,a.title,COALESCE(NULLIF(u.content,''),a.description) AS planned FROM activities a JOIN activity_updates u ON u.activity_id=a.id AND u.year=$2 AND u.month=$3 AND u.kind='plan' WHERE a.department_id=$1 AND u.include_in_document=true ORDER BY a.sort_order,a.created_at")
  .bind(p.department_id).bind(p.year).bind(p.month).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"activity_id":r.get::<Uuid,_>("activity_id"),"title":r.get::<String,_>("title"),"planned":r.get::<String,_>("planned")})).collect())))
}

#[derive(Deserialize)] struct ReorderActivities {department_id:Uuid,ordered_ids:Vec<Uuid>}
async fn reorder_activities(State(s):State<App>,h:HeaderMap,Json(p):Json<ReorderActivities>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !can_manage(&c,p.department_id) {return Err(forbidden());}
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
 let c=who(&s,&h)?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 if !can_manage(&c,dept) {return Err(forbidden());}
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
 let c=who(&s,&h)?; if !can_read(&c,p.department_id) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 let row=sqlx::query("SELECT id,status,submitted_at FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind=$4").bind(p.department_id).bind(p.year).bind(p.month).bind(p.kind).fetch_optional(&s.db).await.map_err(db)?;
 Ok(Json(match row {Some(r)=>json!({"id":r.get::<Uuid,_>("id"),"status":r.get::<String,_>("status"),"submitted_at":r.get::<Option<chrono::DateTime<Utc>>,_>("submitted_at")}),None=>json!({"status":"draft"})}))
}
#[derive(Deserialize)] struct SubmitDept {department_id:Uuid,year:i32,month:i32,kind:String,status:String}
async fn save_department_doc(State(s):State<App>,h:HeaderMap,Json(p):Json<SubmitDept>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !can_manage(&c,p.department_id) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 if !["draft","submitted"].contains(&p.status.as_str()) {return Err(bad("Nevaljan status dokumenta"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 // Protect the version sequence from concurrent submissions.
 let doc=sqlx::query("INSERT INTO department_documents(department_id,year,month,kind,status,submitted_by,submitted_at) VALUES($1,$2,$3,$4,'draft',NULL,NULL) ON CONFLICT(department_id,year,month,kind) DO UPDATE SET id=department_documents.id RETURNING id")
  .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_one(&mut *tx).await.map_err(db)?;
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
  sqlx::query("UPDATE department_documents SET status='submitted',submitted_by=$2,submitted_at=COALESCE(submitted_at,now()) WHERE id=$1")
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
 let c=who(&s,&h)?;if !can_read(&c,p.department_id){return Err(forbidden());}check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT v.id,v.version,v.snapshot,v.submitted_at,u.full_name AS author FROM department_document_versions v JOIN department_documents d ON d.id=v.department_document_id JOIN users u ON u.id=v.submitted_by WHERE d.department_id=$1 AND d.year=$2 AND d.month=$3 AND d.kind=$4 ORDER BY v.version DESC")
 .bind(p.department_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"version":r.get::<i32,_>("version"),
 "snapshot":r.get::<Value,_>("snapshot"),"submitted_at":r.get::<chrono::DateTime<Utc>,_>("submitted_at"),"author":r.get::<String,_>("author")})).collect())))
}

// Collaboration is scoped to the activity's department. Only thread members
// and department managers/directors can read messages; all message writes
// are timestamped, append-only.
async fn activity_people(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 if !can_read(&c,dept){return Err(forbidden());}
 let rows=sqlx::query("SELECT id,full_name,role FROM users WHERE active=true AND (department_id=$1 OR role IN ('director','admin')) ORDER BY full_name")
 .bind(dept).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"name":r.get::<String,_>("full_name"),"role":r.get::<String,_>("role")})).collect())))
}
async fn thread_access(s:&App,c:&Claims,id:Uuid)->ResultApi<(Uuid,Uuid,bool)> {
 let row=sqlx::query("SELECT t.activity_id,a.department_id FROM activity_threads t JOIN activities a ON a.id=t.activity_id WHERE t.id=$1")
  .bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Razgovor ne postoji"))?;
 let a:Uuid=row.get("activity_id");let dept:Uuid=row.get("department_id");
 let supervisor=director(c)||(c.role=="manager"&&c.department_id==Some(dept));
 let member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM activity_thread_participants WHERE thread_id=$1 AND user_id=$2)")
  .bind(id).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 if !supervisor&&!member {return Err(forbidden());}
 Ok((a,dept,supervisor))
}
async fn threads_list(State(s):State<App>,h:HeaderMap,Path(activity_id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1")
  .bind(activity_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 if !can_read(&c,dept){return Err(forbidden());}
 let supervisor=director(&c)||(c.role=="manager"&&c.department_id==Some(dept));
 let rows=sqlx::query("SELECT t.id,t.title,t.status,t.created_at,t.updated_at,u.full_name AS created_name, (SELECT COUNT(*) FROM activity_thread_messages m WHERE m.thread_id=t.id) AS message_count FROM activity_threads t JOIN users u ON u.id=t.created_by WHERE t.activity_id=$1 AND ($2 OR EXISTS(SELECT 1 FROM activity_thread_participants p WHERE p.thread_id=t.id AND p.user_id=$3)) ORDER BY t.updated_at DESC")
  .bind(activity_id).bind(supervisor).bind(c.sub).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"status":r.get::<String,_>("status"),"created_name":r.get::<String,_>("created_name"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"updated_at":r.get::<chrono::DateTime<Utc>,_>("updated_at"),"message_count":r.get::<i64,_>("message_count")})).collect())))
}
#[derive(Deserialize)] struct StartThread {title:String,participants:Vec<Uuid>,message:Option<String>}
async fn thread_create(State(s):State<App>,h:HeaderMap,Path(activity_id):Path<Uuid>,Json(p):Json<StartThread>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;
 let dept:Uuid=sqlx::query_scalar("SELECT department_id FROM activities WHERE id=$1").bind(activity_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Aktivnost ne postoji"))?;
 if !can_manage(&c,dept){return Err(forbidden());}
 if p.title.trim().is_empty()||p.title.len()>250||p.participants.len()>30||p.message.as_ref().is_some_and(|m|m.len()>12000){return Err(bad("Nevaljan naziv, broj sudionika ili duljina poruke"));}
 let mut participants:HashSet<Uuid>=p.participants.into_iter().collect();participants.insert(c.sub);
 let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE active=true AND id = ANY($1) AND (department_id=$2 OR role IN ('admin','director'))")
  .bind(participants.iter().copied().collect::<Vec<_>>()).bind(dept).fetch_one(&s.db).await.map_err(db)?;
 if count!=participants.len() as i64{return Err(bad("Odabrani korisnik nema pristup ovom odjelu"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 let id:Uuid=sqlx::query_scalar("INSERT INTO activity_threads(activity_id,title,created_by) VALUES($1,$2,$3) RETURNING id").bind(activity_id).bind(p.title.trim()).bind(c.sub).fetch_one(&mut *tx).await.map_err(db)?;
 for person in participants {
  sqlx::query("INSERT INTO activity_thread_participants(thread_id,user_id,added_by) VALUES($1,$2,$3)")
   .bind(id).bind(person).bind(c.sub).execute(&mut *tx).await.map_err(db)?;
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
 let c=who(&s,&h)?;thread_access(&s,&c,id).await?;
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
 let c=who(&s,&h)?;thread_access(&s,&c,id).await?;
 if p.body.trim().is_empty()||p.body.len()>12000{return Err(bad("Poruka mora sadržavati između 1 i 12000 znakova"));}
 let status:String=sqlx::query_scalar("SELECT status FROM activity_threads WHERE id=$1").bind(id).fetch_one(&s.db).await.map_err(db)?;
 if status=="done"{return Err(bad("Završeni razgovor prvo ponovno otvori"));}
 let message_id:Uuid=sqlx::query_scalar("INSERT INTO activity_thread_messages(thread_id,author_id,body) VALUES($1,$2,$3) RETURNING id").bind(id).bind(c.sub).bind(p.body.trim()).fetch_one(&s.db).await.map_err(db)?;
 sqlx::query("UPDATE activity_threads SET updated_at=now() WHERE id=$1").bind(id).execute(&s.db).await.map_err(db)?;
 audit(&s,&c,"thread.message",Some(id)).await;
 Ok(Json(json!({"id":message_id})))
}
#[derive(Deserialize)] struct ThreadParticipant {user_id:Uuid}
async fn thread_add_person(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<ThreadParticipant>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;let (_,dept,supervisor)=thread_access(&s,&c,id).await?;if !supervisor{return Err(forbidden());}
 let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1 AND active=true AND (department_id=$2 OR role IN ('director','admin')))")
 .bind(p.user_id).bind(dept).fetch_one(&s.db).await.map_err(db)?;
 if !allowed {return Err(bad("Korisnik nema pristup ovom odjelu"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 let inserted=sqlx::query("INSERT INTO activity_thread_participants(thread_id,user_id,added_by) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
 .bind(id).bind(p.user_id).bind(c.sub).execute(&mut *tx).await.map_err(db)?.rows_affected();
 if inserted>0 {
  sqlx::query("INSERT INTO activity_thread_events(thread_id,actor_id,event_type,payload) VALUES($1,$2,'participant_added',$3)")
   .bind(id).bind(c.sub).bind(json!({"user_id":p.user_id})).execute(&mut *tx).await.map_err(db)?;
  sqlx::query("UPDATE activity_threads SET updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(db)?;
 }
 tx.commit().await.map_err(db)?;Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)] struct ThreadStatus {status:String}
async fn thread_status(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<ThreadStatus>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;let (_,_,supervisor)=thread_access(&s,&c,id).await?;if !supervisor{return Err(forbidden());}
 if !["open","done"].contains(&p.status.as_str()){return Err(bad("Nevaljan status"));}
 let mut tx=s.db.begin().await.map_err(db)?;
 sqlx::query("UPDATE activity_threads SET status=$2,updated_at=now() WHERE id=$1").bind(id).bind(&p.status).execute(&mut *tx).await.map_err(db)?;
 sqlx::query("INSERT INTO activity_thread_events(thread_id,actor_id,event_type,payload) VALUES($1,$2,'status_changed',$3)").bind(id).bind(c.sub).bind(json!({"status":p.status})).execute(&mut *tx).await.map_err(db)?;
 tx.commit().await.map_err(db)?;audit(&s,&c,"thread.status",Some(id)).await;Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)] struct SectorFreshnessFilter {year:i32,month:i32,kind:String}
async fn sector_freshness(State(s):State<App>,h:HeaderMap,Query(p):Query<SectorFreshnessFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?;if !director(&c){return Err(forbidden());}check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT dd.department_id, COALESCE(MAX(v.version),0) AS version FROM department_documents dd LEFT JOIN department_document_versions v ON v.department_document_id=dd.id WHERE dd.year=$1 AND dd.month=$2 AND dd.kind=$3 GROUP BY dd.department_id")
 .bind(p.year).bind(p.month).bind(&p.kind).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"department_id":r.get::<Uuid,_>("department_id"),"version":r.get::<i64,_>("version")})).collect())))
}
#[derive(Deserialize)] struct SectorFilter {year:i32,month:i32,kind:String}
async fn sector_docs(State(s):State<App>,h:HeaderMap,Query(p):Query<SectorFilter>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !director(&c) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 let rows=sqlx::query("SELECT id,title,status,revision,created_at,snapshot,introduction,conclusion FROM sector_documents WHERE year=$1 AND month=$2 AND kind=$3 ORDER BY revision DESC").bind(p.year).bind(p.month).bind(p.kind).fetch_all(&s.db).await.map_err(db)?;
 Ok(Json(Value::Array(rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"title":r.get::<String,_>("title"),"status":r.get::<String,_>("status"),"revision":r.get::<i32,_>("revision"),"created_at":r.get::<chrono::DateTime<Utc>,_>("created_at"),"snapshot":r.get::<Value,_>("snapshot"),"introduction":r.get::<String,_>("introduction"),"conclusion":r.get::<String,_>("conclusion")})).collect())))
}
#[derive(Deserialize)] struct BuildSector {year:i32,month:i32,kind:String,department_ids:Vec<Uuid>,introduction:Option<String>,conclusion:Option<String>}
async fn build_sector(State(s):State<App>,h:HeaderMap,Json(p):Json<BuildSector>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !director(&c) {return Err(forbidden());} check_period(p.year,p.month,&p.kind)?;
 if p.department_ids.is_empty() {return Err(bad("Odaberite barem jedan odjel"));}
 let mut sections=Vec::new();
 for dept_id in &p.department_ids {
  let d=sqlx::query("SELECT name,code FROM departments WHERE id=$1").bind(dept_id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Nepoznat odjel"))?;
  let accepted:Option<String>=sqlx::query_scalar("SELECT status FROM department_documents WHERE department_id=$1 AND year=$2 AND month=$3 AND kind=$4").bind(dept_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_optional(&s.db).await.map_err(db)?;
  if accepted.as_deref()!=Some("submitted") {return Err(bad(format!("Odjel {} još nije predao dokument",d.get::<String,_>("name"))));}
  // Activities are included by default even before a monthly update is saved.
  // A monthly update supplements the permanent description and may exclude the item.
  let rows=sqlx::query("SELECT a.id,a.parent_id,a.title,a.description,a.type, COALESCE(u.content, '') AS content, COALESCE(u.progress, 'in_progress') AS progress FROM activities a LEFT JOIN activity_updates u ON u.activity_id=a.id AND u.year=$2 AND u.month=$3 AND u.kind=$4 WHERE a.department_id=$1 AND COALESCE(u.include_in_document, true)=true ORDER BY a.sort_order,a.created_at")
   .bind(dept_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_all(&s.db).await.map_err(db)?;
  if rows.is_empty() {
   return Err(bad(format!("Odjel {} nema nijednu stavku uključenu u dokument. Dodajte aktivnost ili provjerite oznaku 'U dokument'.",d.get::<String,_>("name"))));
  }
  let items:Vec<Value>=rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"parent_id":r.get::<Option<Uuid>,_>("parent_id"),"title":r.get::<String,_>("title"),"description":r.get::<String,_>("description"),"type":r.get::<String,_>("type"),"content":r.get::<String,_>("content"),"progress":r.get::<String,_>("progress")})).collect();
  let source_version:Option<i32>=sqlx::query_scalar("SELECT MAX(v.version) FROM department_document_versions v JOIN department_documents dd ON dd.id=v.department_document_id WHERE dd.department_id=$1 AND dd.year=$2 AND dd.month=$3 AND dd.kind=$4")
   .bind(dept_id).bind(p.year).bind(p.month).bind(&p.kind).fetch_one(&s.db).await.map_err(db)?;
  sections.push(json!({"department_id":dept_id,"code":d.get::<String,_>("code"),"name":d.get::<String,_>("name"),"source_version":source_version,"items":items}));
 }
 let title=format!("{} značajnijih aktivnosti za {:02}.{}.",if p.kind=="plan" {"PLAN"}else{"IZVJEŠĆE"},p.month,p.year);
 let snapshot=json!({"year":p.year,"month":p.month,"kind":p.kind,"title":title,"sections":sections});
 let revision:i32=sqlx::query_scalar("SELECT COALESCE(MAX(revision),0)+1 FROM sector_documents WHERE year=$1 AND month=$2 AND kind=$3").bind(p.year).bind(p.month).bind(&p.kind).fetch_one(&s.db).await.map_err(db)?;
 let id:Uuid=sqlx::query_scalar("INSERT INTO sector_documents(year,month,kind,title,introduction,conclusion,snapshot,revision,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id").bind(p.year).bind(p.month).bind(&p.kind).bind(&title).bind(p.introduction.unwrap_or_default()).bind(p.conclusion.unwrap_or_default()).bind(snapshot).bind(revision).bind(c.sub).fetch_one(&s.db).await.map_err(db)?;
 audit(&s,&c,"sector.build",Some(id)).await; Ok(Json(json!({"id":id,"revision":revision})))
}

// Editing a sector draft changes ONLY its snapshot, never the underlying activities
// or submitted department reports. Locked revisions stay immutable.
#[derive(Deserialize)] struct EditSector {snapshot:Value,introduction:String,conclusion:String}
async fn edit_sector(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>,Json(p):Json<EditSector>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !director(&c) {return Err(forbidden());}
 let row=sqlx::query("SELECT snapshot,status FROM sector_documents WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Dokument nije pronađen"))?;
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
 let n=sqlx::query("UPDATE sector_documents SET snapshot=$2,introduction=$3,conclusion=$4 WHERE id=$1 AND status='draft'")
  .bind(id).bind(snapshot).bind(p.introduction).bind(p.conclusion).execute(&s.db).await.map_err(db)?.rows_affected();
 if n==0 {return Err(bad("Dokument je u međuvremenu zaključan"));}
 audit(&s,&c,"sector.edit",Some(id)).await;Ok(Json(json!({"ok":true})))
}

async fn lock_sector(State(s):State<App>,h:HeaderMap,Path(id):Path<Uuid>)->ResultApi<Json<Value>> {
 let c=who(&s,&h)?; if !director(&c) {return Err(forbidden());}
 let n=sqlx::query("UPDATE sector_documents SET status='locked',locked_at=now() WHERE id=$1 AND status='draft'").bind(id).execute(&s.db).await.map_err(db)?.rows_affected();
 if n==0 {return Err(bad("Dokument nije nacrt ili ne postoji"));} audit(&s,&c,"sector.lock",Some(id)).await;
 Ok(Json(json!({"ok":true})))
}
async fn export_doc(State(s):State<App>,h:HeaderMap,Path((id,format)):Path<(Uuid,String)>)->ResultApi<Response> {
 let c=who(&s,&h)?; if !director(&c) {return Err(forbidden());}
 if format!="docx" && format!="pdf" {return Err(bad("Nepodržani format"));}
 let r=sqlx::query("SELECT title,introduction,conclusion,snapshot,revision FROM sector_documents WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(db)?.ok_or_else(||bad("Nepostojeći dokument"))?;
 let payload=json!({"title":r.get::<String,_>("title"),"introduction":r.get::<String,_>("introduction"),"conclusion":r.get::<String,_>("conclusion"),"snapshot":r.get::<Value,_>("snapshot")});
 let resp=s.client.post(format!("{}/render/{}",s.doc_url,format)).json(&payload).send().await.map_err(|e|ErrApi(StatusCode::BAD_GATEWAY,e.to_string()))?;
 if !resp.status().is_success() {return Err(ErrApi(StatusCode::BAD_GATEWAY,resp.text().await.unwrap_or_default()));}
 let bytes=resp.bytes().await.map_err(|e|ErrApi(StatusCode::BAD_GATEWAY,e.to_string()))?;
 let mime=if format=="pdf" {"application/pdf"} else {"application/vnd.openxmlformats-officedocument.wordprocessingml.document"};
 Ok(Response::builder().status(StatusCode::OK).header(header::CONTENT_TYPE,mime).header(header::CONTENT_DISPOSITION,format!("attachment; filename=reportflow-{}.{}",id,format)).body(Body::from(bytes)).unwrap())
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
  .route("/api/departments",get(departments)).route("/api/users",get(users).post(create_user))
  .route("/api/dashboard",get(dashboard))
  .route("/api/activities",get(activities).post(create_activity)).route("/api/activities/{id}",patch(edit_activity))
  .route("/api/activity-search",get(search_activities))
  .route("/api/activities/{id}/history",get(activity_history))
  .route("/api/activities/{id}/people",get(activity_people))
  .route("/api/activities/{id}/threads",get(threads_list).post(thread_create))
  .route("/api/threads/{id}",get(thread_detail))
  .route("/api/threads/{id}/messages",post(thread_send))
  .route("/api/threads/{id}/participants",post(thread_add_person))
  .route("/api/threads/{id}/status",patch(thread_status))
  .route("/api/updates",get(updates).post(save_update))
  .route("/api/updates/carry-forward",post(carry_forward))
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
