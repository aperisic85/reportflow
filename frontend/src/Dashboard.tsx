import {useCallback,useEffect,useState} from 'react';
import {api,type User} from './api';
type Department={id:string;name:string;total:number;active:number;monthly_updates:number;report_status:string;plan_status:string};
type Recent={activity_id:string;title:string;department_name:string;updated_at:string;year:number;month:number;kind:string};
type Data={departments:Department[];recent:Recent[]};
type Target='entries'|'activities'|'department'|'sector'|'history';
type Props={user:User;year:number;month:number;onNavigate:(target:Target,departmentId?:string)=>void};
export default function Dashboard({user,year,month,onNavigate}:Props){
 const [data,setData]=useState<Data|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 const load=useCallback(async()=>{
  setBusy(true);setError('');
  try{setData(await api<Data>('/dashboard?'+new URLSearchParams({year:String(year),month:String(month)})));}
  catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}
 },[year,month]);
 useEffect(()=>{void load();},[load]);
 const elevated=user.role==='director'||user.role==='admin';
 const departments=data?.departments??[];
 const total=departments.reduce((n,d)=>n+d.total,0);
 const monthName=new Date(year,month-1,1).toLocaleDateString('hr-HR',{month:'long',year:'numeric'});
 return <div className="simple-home">
  <div className="simple-home-head"><div><h2>Dobro došli</h2><p>{monthName} · {elevated?'Sektor sigurnosti plovidbe':'Moj odjel'}</p></div>
   <button className="outline" onClick={()=>void load()} disabled={busy}>Osvježi</button></div>
  {error&&<p role="alert" className="alert danger">{error}</p>}
  {!data&&!error&&<p>Učitavanje...</p>}
  {data&&<>
   <div className="simple-home-summary"><strong>{total}</strong> aktivnosti u registru <span>·</span> <strong>{departments.reduce((n,d)=>n+d.monthly_updates,0)}</strong> mjesečnih zapisa <span>·</span> <strong>{departments.filter(d=>d.report_status==='submitted').length}/{departments.length}</strong> predanih izvješća</div>
   <div className="simple-home-shortcuts"><button onClick={()=>onNavigate('entries')}>Mjesečni unos</button><button onClick={()=>onNavigate('activities')}>Registar aktivnosti</button><button onClick={()=>onNavigate('history')}>Pretraživanje</button>{elevated&&<button onClick={()=>onNavigate('sector')}>Dokumenti za Upravu</button>}</div>
   <section className="simple-home-section"><h3>{elevated?'Odjeli':'Moj odjel'}</h3>
    <div className="simple-home-table">
     {departments.map(d=><button key={d.id} onClick={()=>onNavigate('department',d.id)}>
      <span className="simple-home-name">{d.name}</span><span>{d.active} u tijeku</span>
      <span>{d.report_status==='submitted'?'Izvješće predano':'Izvješće u izradi'}</span><span className="simple-home-open">Otvori ›</span>
     </button>)}
     {departments.length===0&&<p>Nema odjela za prikaz.</p>}
    </div>
   </section>
   <details className="simple-home-optional"><summary>Posljednje izmjene</summary>
    <ul>{data.recent.map((r,i)=><li key={r.activity_id+'-'+i}><strong>{r.title}</strong> <span>· {r.department_name} · {new Date(r.updated_at).toLocaleDateString('hr-HR')}</span></li>)}</ul>
    {data.recent.length===0&&<p>Nema zabilježenih izmjena.</p>}
   </details>
  </>}
 </div>;
}
