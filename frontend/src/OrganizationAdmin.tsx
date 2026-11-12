import {useCallback,useEffect,useState} from 'react';
import {api,post,patch,type User,type Dept,type Sector} from './api';
type Person={id:string;username:string;full_name:string;role:string;sector_id:string|null;department_id:string|null;active:boolean;created_at:string};
type Editable={full_name:string;role:string;sector_id:string;department_id:string;active:boolean;password:string};
const roles=[{value:'org_admin',label:'Podadmin'},{value:'director',label:'Direktor sektora'},{value:'manager',label:'Voditelj odjela'},{value:'employee',label:'Radnik'}];
function initial(p:Person):Editable{return {full_name:p.full_name,role:p.role,sector_id:p.sector_id??'',department_id:p.department_id??'',active:p.active,password:''};}
export default function OrganizationAdmin({user,onLogout}:{user:User;onLogout:()=>void}){
 const [sectors,setSectors]=useState<Sector[]>([]),[departments,setDepartments]=useState<Dept[]>([]),[users,setUsers]=useState<Person[]>([]);
 const [tab,setTab]=useState<'sectors'|'departments'|'users'>('users'),[editing,setEditing]=useState<string|null>(null);
 const [details,setDetails]=useState<Editable>({full_name:'',role:'employee',sector_id:'',department_id:'',active:true,password:''});
 const [login,setLogin]=useState(''),[newSector,setNewSector]=useState({code:'',name:''}),[newDept,setNewDept]=useState({sector_id:'',code:'',name:''});
 const [error,setError]=useState(''),[notice,setNotice]=useState(''),[busy,setBusy]=useState(false),[isCreating,setIsCreating]=useState(false);
 const refresh=useCallback(async()=>{const [secs,depts,people]=await Promise.all([api<Sector[]>('/sectors'),api<Dept[]>('/departments'),api<Person[]>('/users')]);setSectors(secs);setDepartments(depts);setUsers(people);},[]);
 useEffect(()=>{void refresh().catch(e=>setError(String(e)));},[refresh]);
 const perform=async(f:()=>Promise<unknown>)=>{setBusy(true);setError('');setNotice('');try{await f();await refresh();setNotice('Promjene su spremljene.');}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}};
 const beginNew=()=>{setEditing(null);setIsCreating(true);setLogin('');setDetails({full_name:'',role:'employee',sector_id:sectors[0]?.id??'',department_id:'',active:true,password:''});};
 const edit=(p:Person)=>{setEditing(p.id);setIsCreating(false);setLogin(p.username);setDetails(initial(p));};
 const adminOptions=user.role==='admin'?[{value:'admin',label:'Mega admin'},...roles]:roles;
 const availableDepts=departments.filter(d=>d.sector_id===details.sector_id);
 const chooseRole=(role:string)=>setDetails(d=>({...d,role,sector_id:role==='admin'||role==='org_admin'?'':d.sector_id||sectors[0]?.id||'',department_id:role==='manager'||role==='employee'?d.department_id:''}));
 const savePerson=()=>perform(async()=>{
  const sector_id=['admin','org_admin'].includes(details.role)?null:(details.sector_id||null);
  const department_id=['manager','employee'].includes(details.role)?details.department_id||null:null;
  if(isCreating){await post('/users',{username:login,full_name:details.full_name,password:details.password,role:details.role,sector_id,department_id});}
  else if(editing){await patch('/users/'+editing,{username:login,full_name:details.full_name,role:details.role,sector_id,department_id,active:details.active,password:details.password||null});}
  setIsCreating(false);setEditing(null);setDetails(d=>({...d,password:''}));
 });
 return <div className="org-shell">
  <aside className="org-sidebar"><div className="org-brand"><strong>ReportFlow</strong><small>{user.role==='admin'?'Mega admin':'Podadmin'} · Administracija</small></div>
   <nav><button className={tab==='users'?'active':''} onClick={()=>setTab('users')}>Korisnici</button><button className={tab==='sectors'?'active':''} onClick={()=>setTab('sectors')}>Sektori</button><button className={tab==='departments'?'active':''} onClick={()=>setTab('departments')}>Odjeli</button></nav>
   <button className="org-logout" onClick={onLogout}>Odjava</button></aside>
  <main className="org-main"><header><h1>{tab==='users'?'Korisnici':tab==='sectors'?'Sektori':'Odjeli'}</h1><p>Upravljanje organizacijom i pristupom. Planove i izvješća izrađuju radnici, voditelji i direktori.</p></header>
   {error&&<p className="alert danger" role="alert">{error}</p>}{notice&&<p role="status" className="org-notice">{notice}</p>}
   {tab==='sectors'&&<section className="panel org-panel"><h2>Popis sektora</h2><div className="org-list">{sectors.map(s=><div key={s.id}><strong>{s.name}</strong><small>{s.code}</small></div>)}</div>
    <h3>Novi sektor</h3><form className="org-form" onSubmit={e=>{e.preventDefault();void perform(async()=>{await post('/sectors',newSector);setNewSector({code:'',name:''});});}}>
     <label>Oznaka sektora<input required maxLength={20} value={newSector.code} onChange={e=>setNewSector({...newSector,code:e.target.value})} placeholder="npr. IT"/></label>
     <label>Naziv sektora<input required value={newSector.name} onChange={e=>setNewSector({...newSector,name:e.target.value})} placeholder="Naziv sektora"/></label>
     <button className="primary" disabled={busy}>Dodaj sektor</button></form></section>}
   {tab==='departments'&&<section className="panel org-panel"><h2>Odjeli</h2><div className="org-list">{departments.map(d=><div key={d.id}><strong>{d.name}</strong><small>{sectors.find(s=>s.id===d.sector_id)?.name??'Sektor'} · {d.code}</small></div>)}</div>
    <h3>Novi odjel</h3><form className="org-form" onSubmit={e=>{e.preventDefault();void perform(async()=>{await post('/departments',newDept);setNewDept({sector_id:'',code:'',name:''});});}}>
     <label>Sektor<select value={newDept.sector_id} onChange={e=>setNewDept({...newDept,sector_id:e.target.value})} required><option value="">Odaberi sektor</option>{sectors.map(s=><option key={s.id} value={s.id}>{s.name}</option>)}</select></label>
     <label>Oznaka odjela<input required maxLength={20} value={newDept.code} onChange={e=>setNewDept({...newDept,code:e.target.value})}/></label>
     <label>Naziv odjela<input required value={newDept.name} onChange={e=>setNewDept({...newDept,name:e.target.value})}/></label>
     <button className="primary" disabled={busy}>Dodaj odjel</button></form></section>}
   {tab==='users'&&<section className="panel org-panel"><div className="org-heading"><h2>Korisnici</h2><button className="primary" type="button" onClick={beginNew}>+ Novi korisnik</button></div>
    <div className="org-list">{users.map(p=><div key={p.id}><span><strong>{p.full_name}</strong><small>{p.username} · {p.role==='admin'?'Mega admin':roles.find(r=>r.value===p.role)?.label??p.role} · {p.active?'Aktivan':'Neaktivan'}{p.sector_id?' · '+(sectors.find(s=>s.id===p.sector_id)?.name??'Sektor'):''}{p.department_id?' / '+(departments.find(d=>d.id===p.department_id)?.name??'Odjel'):''}</small></span>{(user.role==='admin'||p.role!=='admin')&&<button type="button" className="outline" onClick={()=>edit(p)}>Uredi</button>}</div>)}</div>
    {(isCreating||editing)&&<form className="org-edit" onSubmit={e=>{e.preventDefault();void savePerson();}}>
     <h3>{isCreating?'Novi korisnik':'Uredi korisnika'}</h3>
     <label>Korisničko ime<input required maxLength={100} value={login} onChange={e=>setLogin(e.target.value)}/></label>
     <label>Ime i prezime<input required value={details.full_name} onChange={e=>setDetails({...details,full_name:e.target.value})}/></label>
     <label>Uloga<select value={details.role} onChange={e=>chooseRole(e.target.value)}>{adminOptions.map(r=><option key={r.value} value={r.value}>{r.label}</option>)}</select></label>
     {!['admin','org_admin'].includes(details.role)&&<label>Sektor<select value={details.sector_id} required onChange={e=>setDetails({...details,sector_id:e.target.value,department_id:''})}><option value="">Odaberi sektor</option>{sectors.map(s=><option key={s.id} value={s.id}>{s.name}</option>)}</select></label>}
     {['manager','employee'].includes(details.role)&&<label>Odjel<select required value={details.department_id} onChange={e=>setDetails({...details,department_id:e.target.value})}><option value="">Odaberi odjel</option>{availableDepts.map(d=><option key={d.id} value={d.id}>{d.name}</option>)}</select></label>}
     <label>{isCreating?'Lozinka (min. 10 znakova)':'Nova lozinka (ostavi prazno ako se ne mijenja)'}<input type="password" required={isCreating} minLength={10} value={details.password} onChange={e=>setDetails({...details,password:e.target.value})}/></label>
     {!isCreating&&<label className="org-check"><input type="checkbox" checked={details.active} onChange={e=>setDetails({...details,active:e.target.checked})}/> Aktivan račun</label>}
     <div className="org-actions"><button className="primary" disabled={busy}>Spremi</button><button type="button" className="outline" onClick={()=>{setEditing(null);setIsCreating(false);}}>Odustani</button></div>
    </form>}
   </section>}
  </main>
 </div>;
}
