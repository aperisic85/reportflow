import {useCallback,useEffect,useState} from 'react';
import {Activity,ArrowRight,BookOpen,CalendarDays,CheckCircle2,Clock3,FileCheck2,FileClock,FolderKanban,RefreshCw} from 'lucide-react';
import {api,type User} from './api';

type DepartmentSummary={
 id:string;name:string;code:string;total:number;active:number;completed:number;
 monthly_updates:number;monthly_completed:number;report_status:'draft'|'submitted';
 plan_status:'draft'|'submitted';
};
type RecentUpdate={
 activity_id:string;title:string;department_name:string;year:number;month:number;
 kind:'report'|'plan';content:string;progress:string;updated_at:string;
};
type DashboardData={year:number;month:number;departments:DepartmentSummary[];recent:RecentUpdate[]};
type DashboardTarget='entries'|'activities'|'department'|'sector'|'history';
type Props={user:User;year:number;month:number;onNavigate:(target:DashboardTarget,departmentId?:string)=>void};
const monthName=(month:number,year:number)=>new Date(year,month-1,1).toLocaleDateString('hr-HR',{month:'long',year:'numeric'});
const total=(values:DepartmentSummary[],key:keyof DepartmentSummary)=>values.reduce((sum,item)=>sum+Number(item[key]||0),0);
const stateLabel=(value:string)=>({completed:'Završeno',in_progress:'U tijeku',postponed:'Odgođeno',not_started:'Nije započeto'} as Record<string,string>)[value]||value;
export default function Dashboard({user,year,month,onNavigate}:Props){
 const [data,setData]=useState<DashboardData|null>(null);
 const [error,setError]=useState('');
 const [loading,setLoading]=useState(false);
 const refresh=useCallback(async()=>{
  setLoading(true);setError('');
  try {setData(await api<DashboardData>('/dashboard?'+new URLSearchParams({year:String(year),month:String(month)}).toString()));}
  catch(e){setError(e instanceof Error?e.message:String(e));}
  finally {setLoading(false);}
 },[year,month]);
 useEffect(()=>{void refresh();},[refresh]);
 const allDepartments=user.role==='admin'||user.role==='director';
 const depts=data?.departments??[];
 const latest=data?.recent??[];
 const submitted=depts.filter(d=>d.report_status==='submitted').length;
 const subtitle=allDepartments?'Pregled svih odjela sektora':'Pregled aktivnosti i izvješća vašeg odjela';
 return <div className="dashboard-page">
  <section className="dashboard-hero">
   <div>
    <p className="dashboard-eyebrow">REPORTFLOW / POČETNA</p>
    <h2>Dobro došli, {user.name.split(' ')[0]}</h2>
    <p>{subtitle} · {monthName(month,year)}</p>
   </div>
   <button className="dashboard-refresh" type="button" onClick={()=>void refresh()} disabled={loading}><RefreshCw size={16}/> {loading?'Učitavanje...':'Osvježi'}</button>
  </section>
  {error&&<div className="alert danger" role="alert">Nije moguće učitati pregled: {error}</div>}
  {!data&&!error&&<section className="panel"><p role="status">Učitavanje početnog pregleda...</p></section>}
  {data&&<>
   <section className="dashboard-kpis" aria-label="Sažetak aktivnosti">
    <div className="dashboard-kpi"><span className="dashboard-kpi-icon"><FolderKanban size={20}/></span><strong>{total(depts,'total')}</strong><span>Aktivnosti u registru</span></div>
    <div className="dashboard-kpi"><span className="dashboard-kpi-icon"><Activity size={20}/></span><strong>{total(depts,'active')}</strong><span>Aktivnosti u tijeku</span></div>
    <div className="dashboard-kpi"><span className="dashboard-kpi-icon"><BookOpen size={20}/></span><strong>{total(depts,'monthly_updates')}</strong><span>Mjesečnih zapisa izvješća</span></div>
    <div className="dashboard-kpi"><span className="dashboard-kpi-icon"><FileCheck2 size={20}/></span><strong>{submitted}/{depts.length}</strong><span>Predanih izvješća odjela</span></div>
   </section>
   <div className="dashboard-columns">
    <section className="panel dashboard-main-panel">
     <div className="dashboard-section-head"><div><h2>{allDepartments?'Stanje po odjelima':'Stanje odjela'}</h2><p>Podaci za odabrano razdoblje</p></div></div>
     {depts.length===0?<p>Nema dostupnih odjela.</p>:<div className="dashboard-departments">
      {depts.map(d=><article className="dashboard-department" key={d.id}>
       <div className="dashboard-department-head">
        <div className="dashboard-department-avatar">{d.code}</div>
        <div className="dashboard-department-title"><strong>{d.name}</strong><small>{d.active} u tijeku · {d.completed} završeno u registru</small></div>
        <button className="outline" type="button" onClick={()=>onNavigate('department',d.id)}>Otvori <ArrowRight size={15}/></button>
       </div>
       <div className="dashboard-department-meta">
        <span><CalendarDays size={15}/> Plan: <b>{d.plan_status==='submitted'?'Predan':'Nacrt'}</b></span>
        <span><FileClock size={15}/> Izvješće: <b>{d.report_status==='submitted'?'Predano':'Nacrt'}</b></span>
        <span><CheckCircle2 size={15}/> {d.monthly_completed}/{d.monthly_updates} zapisa označeno završenima</span>
       </div>
      </article>)}
     </div>}
    </section>
    <section className="panel dashboard-actions">
     <div className="dashboard-section-head"><div><h2>Brze radnje</h2><p>Najčešći zadaci</p></div></div>
     <button type="button" onClick={()=>onNavigate('entries')}><CalendarDays size={19}/><span><strong>Mjesečni unos</strong><small>Unesi realizaciju aktivnosti</small></span><ArrowRight size={16}/></button>
     <button type="button" onClick={()=>onNavigate('activities')}><FolderKanban size={19}/><span><strong>Registar aktivnosti</strong><small>Pregled i dodavanje stavki</small></span><ArrowRight size={16}/></button>
     <button type="button" onClick={()=>onNavigate('history')}><BookOpen size={19}/><span><strong>Povijest i pretraga</strong><small>Pronađi prethodne zapise</small></span><ArrowRight size={16}/></button>
     {allDepartments&&<button type="button" onClick={()=>onNavigate('sector')}><FileCheck2 size={19}/><span><strong>Objedinjavanje za Upravu</strong><small>Pregled sektorskih dokumenata</small></span><ArrowRight size={16}/></button>}
    </section>
   </div>
   <section className="panel dashboard-recent">
    <div className="dashboard-section-head"><div><h2>Nedavne izmjene</h2><p>Posljednjih osam spremljenih mjesečnih zapisa, bez obzira na odabrani mjesec</p></div></div>
    {latest.length===0?<p>Još nema spremljenih mjesečnih zapisa.</p>:<div className="dashboard-recent-list">
     {latest.map((item,i)=><div className="dashboard-recent-item" key={item.activity_id+'-'+item.kind+'-'+item.month+'-'+item.year+'-'+i}>
      <span className="dashboard-recent-dot"><Clock3 size={16}/></span>
      <div><strong>{item.title}</strong><p>{item.department_name} · {monthName(item.month,item.year)} · {item.kind==='plan'?'Plan':'Izvješće'} · {stateLabel(item.progress)}</p>{item.content.trim()&&<small>{item.content.length>190?item.content.slice(0,190)+'…':item.content}</small>}</div>
      <time dateTime={item.updated_at}>{new Date(item.updated_at).toLocaleDateString('hr-HR')}</time>
     </div>)}
    </div>}
   </section>
  </>}
 </div>;
}
