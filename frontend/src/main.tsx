import React,{useCallback,useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {BookOpen,CalendarDays,ClipboardCheck,FileDown,FileText,FolderKanban,LogOut,Plus,RefreshCw,Shield,Users,ArrowUp,ArrowDown,Save,PenLine} from 'lucide-react';
import {api,post,patch,q,download,type User,type Dept,type Activity,type Update,type SectorDoc,type SectorSnapshot,type PlanReference} from './api';
import './style.css';
import HistorySearch from './HistorySearch';
import Dashboard from './Dashboard';
import ActivityConversation from './ActivityConversation';
import Inbox from './Inbox';
import {DepartmentReportPreview,SectorReportPreview,type DepartmentVersion} from './ReportPreview';
import MonthlyEntry from './MonthlyEntry';
const today=new Date();
const kinds=[{value:'report',label:'Izvješće'},{value:'plan',label:'Plan'}];
const statusNames:Record<string,string>={in_progress:'U tijeku',completed:'Završeno',paused:'Pauzirano',cancelled:'Otkazano',postponed:'Odgođeno',not_started:'Nije započeto'};
// Render the registry in its real parent/child order. Broken parent links become roots.
function treeOrder<T extends {id:string;parent_id:string|null;sort_order:number}>(items:T[]):T[]{
 const seen=new Set<string>();const ids=new Set(items.map(i=>i.id));const result:T[]=[];
 const visit=(parent:string|null)=>items.filter(i=>(ids.has(i.parent_id||'')?i.parent_id:null)===parent).sort((a,b)=>a.sort_order-b.sort_order).forEach(i=>{if(seen.has(i.id))return;seen.add(i.id);result.push(i);visit(i.id);});
 visit(null);items.forEach(i=>{if(!seen.has(i.id))result.push(i);});return result;
}
function siblingMove<T extends {id:string;parent_id:string|null;sort_order:number}>(items:T[],id:string,delta:number):string[]{
 const ordered=treeOrder(items);const current=ordered.find(i=>i.id===id);if(!current)return ordered.map(i=>i.id);
 const siblings=ordered.filter(i=>i.parent_id===current.parent_id);const pos=siblings.findIndex(i=>i.id===id);const other=siblings[pos+delta];if(!other)return ordered.map(i=>i.id);
 const priorities=new Map<string,number>();siblings.forEach((i,index)=>priorities.set(i.id,index));priorities.set(current.id,pos+delta);priorities.set(other.id,pos);
 const ranks=new Map(ordered.map((i,index)=>[i.id,index]));
 const children=new Map<string|null,T[]>();for(const i of ordered){const key=i.parent_id;const a=children.get(key)||[];a.push(i);children.set(key,a);}
 const result:string[]=[];const seen=new Set<string>();const walk=(parent:string|null)=>{for(const i of (children.get(parent)||[]).sort((a,b)=>(priorities.get(a.id)??ranks.get(a.id)!)-(priorities.get(b.id)??ranks.get(b.id)!))){if(seen.has(i.id))continue;seen.add(i.id);result.push(i.id);walk(i.id);}};walk(null);ordered.forEach(i=>{if(!seen.has(i.id))result.push(i.id)});return result;
}
function App(){
 const [user,setUser]=useState<User|null>(null),[token,setToken]=useState(sessionStorage.getItem('rf_token'));
 const [error,setError]=useState(''),[notice,setNotice]=useState('');
 const [depts,setDepts]=useState<Dept[]>([]),[dept,setDept]=useState(''),[year,setYear]=useState(today.getFullYear()),[month,setMonth]=useState(today.getMonth()+1),[kind,setKind]=useState('report');
 const [tab,setTab]=useState('home'),[activities,setActivities]=useState<Activity[]>([]),[updates,setUpdates]=useState<Update[]>([]),[docStatus,setDocStatus]=useState('draft');
 const [docs,setDocs]=useState<SectorDoc[]>([]),[people,setPeople]=useState<{username:string;full_name:string;role:string;department_id:string|null}[]>([]);
 const [newTitle,setNewTitle]=useState(''),[newDescription,setNewDescription]=useState(''),[newType,setNewType]=useState('ongoing'),[newParent,setNewParent]=useState('');
 const [newUser,setNewUser]=useState({username:'',full_name:'',password:'',role:'employee',department_id:''});
 const [chosen,setChosen]=useState<string[]>([]),[intro,setIntro]=useState(''),[conclusion,setConclusion]=useState('');
 const [loading,setLoading]=useState(false);
 const [historyActivityId,setHistoryActivityId]=useState<string|null>(null);
 const [planRefs,setPlanRefs]=useState<PlanReference[]>([]);
 const [selectedDraft,setSelectedDraft]=useState<SectorDoc|null>(null);
 const [entrySearch,setEntrySearch]=useState(''),[entryFilter,setEntryFilter]=useState('all');
 const [registrySearch,setRegistrySearch]=useState(''),[registryDetails,setRegistryDetails]=useState<string[]>([]);
 const [showNewActivity,setShowNewActivity]=useState(false);
 const [chatActivityId,setChatActivityId]=useState<string|null>(null),[chatThreadId,setChatThreadId]=useState<string|null>(null);
 const [inboxUnread,setInboxUnread]=useState(0);
 const [versions,setVersions]=useState<DepartmentVersion[]>([]),[selectedVersion,setSelectedVersion]=useState<DepartmentVersion|null>(null);
 const [reportPreview,setReportPreview]=useState<SectorDoc|null>(null);
 const [sourceVersions,setSourceVersions]=useState<Record<string,number>>({});
 const notify=(f:()=>Promise<unknown>)=>{setLoading(true);setError('');setNotice('');void f().then((result)=>setNotice(typeof result==='string'?result:'Promjene su spremljene.')).catch(e=>setError(String(e.message))).finally(()=>setLoading(false));};
 useEffect(()=>{if(!token)return;api<User>('/auth/me').then(setUser).catch(()=>{sessionStorage.removeItem('rf_token');setToken(null);});},[token]);
 useEffect(()=>{if(!user)return;api<Dept[]>('/departments').then(d=>{setDepts(d);setDept(p=>p||user.department_id||d[0]?.id||'');setChosen(d.map(x=>x.id));}).catch(e=>setError(e.message));},[user]);
 const refresh=useCallback(async()=>{
  if(!user||!dept)return;
  const params={department_id:dept,year,month,kind};
  const jobs:Promise<unknown>[]=[api<Activity[]>('/activities'+q({department_id:dept})).then(setActivities),api<Update[]>('/updates'+q(params)).then(setUpdates),api<{status:string}>('/department-documents'+q(params)).then(x=>setDocStatus(x.status))];
  if(user.role==='admin'||user.role==='director')jobs.push(api<SectorDoc[]>('/sector-documents'+q({year,month,kind})).then(setDocs));
  if(user.role==='admin')jobs.push(api<typeof people>('/users').then(setPeople));
  if(kind==='report')jobs.push(api<PlanReference[]>('/plan-reference'+q(params)).then(setPlanRefs));
  else setPlanRefs([]);
  await Promise.all(jobs);
 },[user,dept,year,month,kind]);
 useEffect(()=>{void refresh().catch(e=>setError(e.message));},[refresh]);
 useEffect(()=>{if(!user||!dept)return;api<DepartmentVersion[]>('/department-document-versions'+q({department_id:dept,year,month,kind})).then(setVersions).catch(()=>setVersions([]));},[user,dept,year,month,kind,docStatus]);
 useEffect(()=>{if(!user||!['admin','director'].includes(user.role))return;api<Array<{department_id:string;version:number}>>('/sector-document-freshness'+q({year,month,kind})).then(rows=>setSourceVersions(Object.fromEntries(rows.map(x=>[x.department_id,x.version])))).catch(()=>setSourceVersions({}));},[user,year,month,kind,docs]);
 useEffect(()=>{
  if(!user)return;
  let live=true;
  const check=()=>{void api<Array<{unread:number}>>('/inbox').then(items=>{if(live)setInboxUnread(items.reduce((n,i)=>n+i.unread,0));}).catch(()=>{});};
  check();const timer=window.setInterval(check,30000);
  return ()=>{live=false;window.clearInterval(timer);};
 },[user]);
 if(!token||!user)return <Login onLogin={(v)=>{sessionStorage.setItem('rf_token',v);setToken(v);}} />;
 const canEdit=user.role==='admin'||(user.department_id===dept&&(user.role==='manager'||user.role==='employee'));
 const canSubmit=user.role==='admin'||(user.department_id===dept&&user.role==='manager');
 const isDirector=user.role==='admin'||user.role==='director';
 const selectedDept=depts.find(d=>d.id===dept);
 const dataByActivity=Object.fromEntries(updates.map(u=>[u.activity_id,u]));
 const normalizedEntrySearch=entrySearch.trim().toLocaleLowerCase('hr');
 const visibleEntries=treeOrder(activities).filter(a=>{
  const matchesText=!normalizedEntrySearch||(a.title+' '+a.description).toLocaleLowerCase('hr').includes(normalizedEntrySearch);
  const existing=Boolean(dataByActivity[a.id]);
  return matchesText&&(entryFilter==='all'||(entryFilter==='saved'?existing:!existing));
 });
 const normalizedRegistrySearch=registrySearch.trim().toLocaleLowerCase('hr');
 const visibleRegistry=treeOrder(activities).filter(a=>!normalizedRegistrySearch||
  (a.title+' '+a.description).toLocaleLowerCase('hr').includes(normalizedRegistrySearch));
 const toggleRegistryDetails=(id:string)=>setRegistryDetails(ids=>ids.includes(id)?ids.filter(x=>x!==id):[...ids,id]);
 // The preview must use the same defaults as the backend document generator.
 // A missing monthly record does not mean the registry activity is excluded.
 const includedActivities=activities.filter(a=>dataByActivity[a.id]?.include_in_document!==false);
 const previewContent=(a:Activity)=>{const description=a.description.trim();const monthly=dataByActivity[a.id]?.content?.trim()||'';return {description,monthly:monthly!==description?monthly:''};};
 async function saveActivity(){await post('/activities',{department_id:dept,parent_id:newParent||null,title:newTitle,description:newDescription,type:newType});setNewTitle('');setNewDescription('');setNewParent('');setShowNewActivity(false);await refresh();}
 async function saveUpdate(activityId:string,content:string,progress:string,include:boolean){await post('/updates',{activity_id:activityId,year,month,kind,content,progress,include_in_document:include});await refresh();}
 async function saveUser(){await post('/users',{...newUser,department_id:newUser.department_id||null});setNewUser({username:'',full_name:'',password:'',role:'employee',department_id:''});await refresh();}
 async function build(){await post('/sector-documents',{year,month,kind,department_ids:chosen,introduction:intro,conclusion});setSelectedDraft(null);await refresh();}
 async function carry(){const r=await post<{created:number;source_month:number;source_year:number}>('/updates/carry-forward',{department_id:dept,year,month,kind});await refresh();return `Preneseno ${r.created} nedovršenih stavki iz ${r.source_month}/${r.source_year}. Prethodni mjesečni opisi nisu kopirani.`;}
 async function moveParent(id:string,parent_id:string|null){await post(`/activities/${id}/move`,{parent_id});await refresh();}
 async function moveOrder(id:string,delta:number){await post('/activities/reorder',{department_id:dept,ordered_ids:siblingMove(activities,id,delta)});await refresh();}
 async function saveSectorDraft(snapshot:SectorSnapshot,introduction:string,conclusion:string){if(!selectedDraft)return;await patch(`/sector-documents/${selectedDraft.id}`,{snapshot,introduction,conclusion});setSelectedDraft(null);await refresh();}
 return <div className="layout">
  <aside className="sidebar"><div className="brand"><div className="brand-icon"><BookOpen size={22}/></div><div><strong>ReportFlow</strong><small>Sigurnost plovidbe</small></div></div>
   <nav><button className={tab==='home'?'active':''} onClick={()=>setTab('home')}><BookOpen size={18}/> Početna</button><button className={tab==='entries'?'active':''} onClick={()=>setTab('entries')}><CalendarDays size={18}/> Mjesečni unos</button><button className={tab==='activities'?'active':''} onClick={()=>setTab('activities')}><FolderKanban size={18}/> Registar aktivnosti</button><button className={tab==='inbox'?'active':''} onClick={()=>setTab('inbox')}><BookOpen size={18}/> Inbox {inboxUnread>0&&<span className="inbox-badge">{inboxUnread}</span>}</button><button className={tab==='history'?'active':''} onClick={()=>{setHistoryActivityId(null);setTab('history');}}><BookOpen size={18}/> Povijest i pretraga</button><button className={tab==='department'?'active':''} onClick={()=>setTab('department')}><ClipboardCheck size={18}/> Izvješće odjela</button>{isDirector&&<button className={tab==='sector'?'active':''} onClick={()=>setTab('sector')}><FileText size={18}/> Sektor / Uprava</button>}{user.role==='admin'&&<button className={tab==='users'?'active':''} onClick={()=>setTab('users')}><Users size={18}/> Korisnici</button>}</nav>
   <div className="sidebar-bottom"><div className="avatar">{user.name.slice(0,1).toUpperCase()}</div><div className="identity"><strong>{user.name}</strong><small>{user.role}</small></div><button title="Odjava" className="icon-button" onClick={()=>{sessionStorage.removeItem('rf_token');setToken(null);setUser(null);}}><LogOut size={18}/></button></div>
  </aside>
  <main className="main"><header><div><h1>{({home:'Početni pregled',entries:'Mjesečni unos aktivnosti',activities:'Registar aktivnosti',history:'Povijest i pretraživanje',inbox:'Inbox',department:'Izvješće odjela',sector:'Objedinjavanje za Upravu',users:'Upravljanje korisnicima'} as Record<string,string>)[tab]}</h1></div><button className="outline" onClick={()=>notify(refresh)}><RefreshCw size={16}/> Osvježi</button></header>
   {error&&<div className="alert danger" role="alert">{error}<button onClick={()=>setError('')}>×</button></div>}{notice&&<div className="alert success" role="status">{notice}<button onClick={()=>setNotice('')}>×</button></div>}
   {tab!=='users'&&tab!=='home'&&<div className="filters"><label>Odjel<select value={dept} onChange={e=>setDept(e.target.value)} disabled={user.role==='employee'||user.role==='manager'}>{depts.filter(d=>isDirector||d.id===user.department_id).map(d=><option value={d.id} key={d.id}>{d.name}</option>)}</select></label><label>Mjesec<select value={month} onChange={e=>setMonth(Number(e.target.value))}>{Array.from({length:12},(_,i)=><option key={i} value={i+1}>{new Date(2026,i,1).toLocaleString('hr-HR',{month:'long'})}</option>)}</select></label><label>Godina<input type="number" min="2000" max="2100" value={year} onChange={e=>setYear(Number(e.target.value))}/></label>{tab!=='entries'&&<label>Dokument<select value={kind} onChange={e=>setKind(e.target.value)}>{kinds.map(k=><option key={k.value} value={k.value}>{k.label}</option>)}</select></label>}</div>}
   {tab==='home'&&<Dashboard user={user} year={year} month={month} onNavigate={(target,departmentId)=>{if(departmentId)setDept(departmentId);setTab(target);}}/>}
   {tab==='inbox'&&<Inbox onOpen={(activityId,threadId,departmentId)=>{setDept(departmentId);setChatActivityId(activityId);setChatThreadId(threadId);setTab('activities');setInboxUnread(0);}}/>}
   {tab==='history'&&<HistorySearch user={user} depts={depts} initialActivityId={historyActivityId}/>}
   {tab==='entries'&&<section className="panel ux-panel">
    <div className="ux-intro">
     <div><h2>{kind==='plan'?'Izrada mjesečnog plana':'Unos mjesečne realizacije'}</h2>
      <p>{selectedDept?.name} · {new Date(year,month-1,1).toLocaleDateString('hr-HR',{month:'long',year:'numeric'})}</p></div>
     <span className={'chip '+(docStatus==='submitted'?'green':'amber')}>{docStatus==='submitted'?'Dokument je predan':'Dokument u izradi'}</span>
    </div>
    <div className="ux-kind-switch" role="group" aria-label="Odabir vrste unosa">
     <button type="button" className={kind==='plan'?'active':''} aria-pressed={kind==='plan'} onClick={()=>setKind('plan')}>Plan</button>
     <button type="button" className={kind==='report'?'active':''} aria-pressed={kind==='report'} onClick={()=>setKind('report')}>Izvješće</button>
    </div>
    <p className="simple-guidance">{kind==='plan'?'Upiši što planiraš napraviti u ovom mjesecu.':'Upiši što je stvarno napravljeno u ovom mjesecu i označi status.'}</p>
    <p className="simple-counter">{updates.length} od {activities.length} aktivnosti ima mjesečni zapis.</p>
    <div className="ux-toolbar">
     <label>Pronađi aktivnost<input value={entrySearch} onChange={e=>setEntrySearch(e.target.value)} placeholder="Pretraži naziv ili opis aktivnosti"/></label>
     <label>Prikaži<select value={entryFilter} onChange={e=>setEntryFilter(e.target.value)}>
      <option value="all">Sve aktivnosti</option><option value="missing">Bez mjesečnog unosa</option><option value="saved">Sa spremljenim unosom</option>
     </select></label>
    </div>
    {kind==='report'&&canSubmit&&<details className="simple-options"><summary>Dodatne radnje</summary><p>Možeš prenijeti nedovršene aktivnosti iz prošlog mjeseca bez prepisivanja njihovih opisa.</p><button className="outline" disabled={loading||docStatus==='submitted'} onClick={()=>notify(carry)}>Prenesi nedovršene</button></details>}
    {activities.length===0?<Empty text="Nema aktivnosti. Prvo dodaj aktivnost u Registru aktivnosti."/>:
     visibleEntries.length===0?<Empty text="Nema aktivnosti koje odgovaraju filtrima."/>:
     <div className="ux-entry-list">{visibleEntries.map(a=><MonthlyEntry
      key={`${a.id}-${year}-${month}-${kind}`}
      item={a} kind={kind as 'plan'|'report'}
      planReference={kind==='report'?planRefs.find(r=>r.activity_id===a.id)?.planned:undefined}
      initial={dataByActivity[a.id]} disabled={!canEdit||docStatus==='submitted'||(user.role==='employee'&&a.created_by!==user.id)}
      onSave={(c,p,i)=>notify(()=>saveUpdate(a.id,c,p,i))} busy={loading}/>)}</div>}
   </section>}
   {tab==='activities'&&<><section className="panel ux-panel">
    <div className="ux-intro"><div><h2>Registar aktivnosti</h2>
     <p>Trajni popis poslova i projekata · {activities.length} aktivnosti</p></div>
     {canEdit&&<button type="button" className="primary registry-create-trigger" aria-expanded={showNewActivity} onClick={()=>setShowNewActivity(v=>!v)}><Plus size={17}/>{showNewActivity?'Zatvori unos':'Nova aktivnost'}</button>}
    </div>
    {showNewActivity&&canEdit&&<form onSubmit={e=>{e.preventDefault();notify(saveActivity);}} className="registry-create-form form-grid">
     <h3 className="wide">Nova aktivnost</h3>
     <label className="wide">Naziv aktivnosti<input required value={newTitle} onChange={e=>setNewTitle(e.target.value)} placeholder="Npr. Modernizacija VHF sustava"/></label>
     <label className="wide">Opis aktivnosti<textarea rows={3} value={newDescription} onChange={e=>setNewDescription(e.target.value)} placeholder="Kratki opis posla ili projekta"/></label>
     <label>Vrsta aktivnosti<select value={newType} onChange={e=>setNewType(e.target.value)}><option value="ongoing">Trajna aktivnost</option><option value="project">Projekt</option><option value="one_off">Jednokratna aktivnost</option></select></label>
     <label>Nadređena aktivnost (neobavezno)<select value={newParent} onChange={e=>setNewParent(e.target.value)}><option value="">Bez nadređene aktivnosti</option>{activities.map(a=><option value={a.id} key={a.id}>{a.title}</option>)}</select></label>
     <div className="wide registry-create-actions"><button className="primary" type="submit" disabled={loading}><Plus size={17}/> Spremi aktivnost</button><button className="outline" type="button" onClick={()=>setShowNewActivity(false)}>Odustani</button></div>
    </form>}
    <div className="ux-toolbar"><label>Pronađi aktivnost<input value={registrySearch} onChange={e=>setRegistrySearch(e.target.value)} placeholder="Naziv ili opis projekta / aktivnosti"/></label>
     <button type="button" className="outline" onClick={()=>setTab('entries')}>Mjesečni unos</button>
    </div>
    {visibleRegistry.length===0?<Empty text="Nema aktivnosti koje odgovaraju pretrazi."/>:
     <div className="ux-registry-list">{visibleRegistry.map(a=><article className="ux-registry-item" key={a.id}>
      <div className="ux-registry-top"><div className="ux-registry-name">
       <span className="ux-parent-indicator">{a.parent_id?'↳':'•'}</span>
       <div><button type="button" className="activity-title-link" onClick={()=>{setChatActivityId(a.id);setChatThreadId(null);}}>{a.title}</button><small>{a.created_at?'Kreirano '+new Date(a.created_at).toLocaleString('hr-HR')+' · ':''}{a.type==='project'?'Projekt':a.type==='ongoing'?'Trajna aktivnost':'Jednokratna aktivnost'}</small></div>
      </div>
      <div className="ux-registry-actions">
       <span className="simple-registry-status">{statusNames[a.status]}</span>
       <button type="button" className="outline compact" onClick={()=>{setChatActivityId(a.id);setChatThreadId(null);}}>Chat</button>
       <button type="button" className="outline compact" onClick={()=>toggleRegistryDetails(a.id)}>{registryDetails.includes(a.id)?'Sakrij detalje':'Detalji'}</button>
       <button type="button" className="outline compact" onClick={()=>{setHistoryActivityId(a.id);setTab('history');}}>Povijest</button>
      </div></div>
      {chatActivityId===a.id&&<ActivityConversation key={a.id+'-'+(chatThreadId??'')} activityId={a.id} canManage={canSubmit} openThread={chatThreadId} onClose={()=>{setChatActivityId(null);setChatThreadId(null);}}/>}
      {a.description&&registryDetails.includes(a.id)&&<p className="ux-registry-description">{a.description}</p>}
      {registryDetails.includes(a.id)&&<div className="ux-registry-details">
       {canSubmit&&<label>Nadređena aktivnost<select value={a.parent_id||''} disabled={loading} onChange={e=>notify(()=>moveParent(a.id,e.target.value||null))}>
        <option value="">Bez nadređene aktivnosti</option>
        {activities.filter(x=>x.id!==a.id).map(x=><option key={x.id} value={x.id}>{x.title}</option>)}
       </select></label>}
       {canSubmit&&<div className="ux-registry-order"><span>Redoslijed prikaza</span>
        <button className="outline compact" title="Pomakni gore" disabled={loading} onClick={()=>notify(()=>moveOrder(a.id,-1))}><ArrowUp size={15}/> Gore</button>
        <button className="outline compact" title="Pomakni dolje" disabled={loading} onClick={()=>notify(()=>moveOrder(a.id,1))}><ArrowDown size={15}/> Dolje</button>
       </div>}
      </div>}
     </article>)}</div>}
   </section></>}
   {tab==='department'&&<section className="panel"><div className="panel-head"><div><h2>{kind==='plan'?'Plan':'Izvješće'} odjela</h2><p>Predajom se zaključava uređivanje mjesečnih zapisa dok voditelj ne vrati dokument u nacrt.</p></div><span className={'chip '+(docStatus==='submitted'?'green':'amber')}>{docStatus==='submitted'?'Predano':'Nacrt'}</span></div><div className="summary"><div><strong>{activities.length}</strong><small>Aktivnosti u registru</small></div><div><strong>{updates.length}</strong><small>Mjesečnih zapisa</small></div><div><strong>{includedActivities.length}</strong><small>U dokumentu</small></div></div><h3>Pregled sadržaja</h3>{includedActivities.length?includedActivities.map(a=><div className="activity-row" key={a.id}><div><strong>{a.parent_id?'↳ ':''}{a.title}</strong>{previewContent(a).description&&<p>{previewContent(a).description}</p>}{previewContent(a).monthly&&<p><strong>{kind==='plan'?'Mjesečni plan:':'Mjesečna realizacija:'}</strong> {previewContent(a).monthly}</p>}</div></div>):<Empty text="Nema aktivnosti uključenih u dokument. Dodajte aktivnost ili označite 'U dokument'."/>}{canSubmit&&<div className="actions"><button className="primary" disabled={loading||(docStatus!=='submitted'&&includedActivities.length===0)} onClick={()=>notify(async()=>{await post('/department-documents',{department_id:dept,year,month,kind,status:docStatus==='submitted'?'draft':'submitted'});await refresh();})}>{docStatus==='submitted'?'Vrati u nacrt':kind==='plan'?'Predaj plan odjela':'Predaj izvješće odjela'}</button></div>}<div className="report-version-history">
  <h3>Predane verzije</h3>
  {versions.length===0?<p>Još nema predanih verzija.</p>:versions.map(v=><div className="report-version-row" key={v.id}>
   <span>Verzija {v.version} · {v.author} · {new Date(v.submitted_at).toLocaleString('hr-HR')}</span>
   <button type="button" className="outline compact" onClick={()=>setSelectedVersion(v)}>Otvori HTML</button>
  </div>)}
 </div>
 {selectedVersion&&<DepartmentReportPreview version={selectedVersion} onClose={()=>setSelectedVersion(null)}/>}
</section>}
   {tab==='sector'&&isDirector&&<><section className="panel"><div className="panel-head"><div><h2>Izrada završnog dokumenta</h2><p>Objedinjuju se predani dokumenti odjela. Izrada stvara novu reviziju, koju možete urediti prije zaključavanja.</p></div><Shield size={22}/></div><div className="checkbox-list">{depts.map(d=><label key={d.id}><input type="checkbox" checked={chosen.includes(d.id)} onChange={e=>setChosen(p=>e.target.checked?[...p,d.id]:p.filter(x=>x!==d.id))}/>{d.name}</label>)}</div><label>Uvod<textarea rows={3} value={intro} onChange={e=>setIntro(e.target.value)} placeholder="Neobavezna napomena direktora sektora"/></label><label>Zaključak<textarea rows={3} value={conclusion} onChange={e=>setConclusion(e.target.value)} placeholder="Neobavezni zaključak"/></label><div className="actions"><button className="primary" disabled={loading||chosen.length===0} onClick={()=>notify(build)}>Izradi objedinjeni dokument</button></div></section><section className="panel"><h2>Verzije dokumenta</h2>{docs.length===0?<Empty text="Za ovo razdoblje još nema objedinjenih dokumenata."/>:docs.map(d=><div className="document-row" key={d.id}><div><strong>{d.title}</strong><p>Revizija {d.revision} · {d.snapshot.sections.length} odjela · {d.snapshot.sections.reduce((n,s)=>n+s.items.length,0)} stavki · {new Date(d.created_at).toLocaleDateString('hr-HR')}</p></div><span className={'chip '+(d.status==='locked'?'green':'amber')}>{d.status==='locked'?'Zaključano':'Nacrt'}</span>{d.snapshot.sections.some(sec=>sec.source_version!=null&&(sourceVersions[sec.department_id]??0)>sec.source_version)&&<span className="chip amber" title="Jedan ili više odjela predali su novu verziju nakon objedinjavanja">Novija verzija odjela</span>}<div className="document-actions"><button className="outline" onClick={()=>setReportPreview(d)}>HTML pregled</button>{d.status==='draft'&&<button className="outline" onClick={()=>setSelectedDraft(d)}><PenLine size={16}/> Uredi / pregledaj</button>}<button className="outline" onClick={()=>notify(()=>download(d.id,'docx'))}><FileDown size={16}/> Word</button><button className="outline" onClick={()=>notify(()=>download(d.id,'pdf'))}><FileDown size={16}/> PDF</button>{d.status==='draft'&&<button className="primary" onClick={()=>notify(async()=>{await post(`/sector-documents/${d.id}/lock`,{});setSelectedDraft(null);await refresh();})}>Zaključaj</button>}</div></div>)}</section>{selectedDraft&&<SectorEditor key={selectedDraft.id} doc={selectedDraft} busy={loading} onCancel={()=>setSelectedDraft(null)} onSave={(snap,opening,closing)=>notify(()=>saveSectorDraft(snap,opening,closing))}/>}</>}
   {tab==='sector'&&reportPreview&&<SectorReportPreview doc={reportPreview} onClose={()=>setReportPreview(null)}/>}
   {tab==='users'&&user.role==='admin'&&<><section className="panel"><h2>Korisnici</h2><div className="activities-list">{people.map(p=><div className="activity-row" key={p.username}><div><strong>{p.full_name}</strong><p>{p.username} · {p.role} · {depts.find(d=>d.id===p.department_id)?.name||'Razina sektora'}</p></div></div>)}</div></section><section className="panel"><h2>Dodaj korisnika</h2><form className="form-grid" onSubmit={e=>{e.preventDefault();notify(saveUser);}}><label>Kor. ime<input required value={newUser.username} onChange={e=>setNewUser({...newUser,username:e.target.value})}/></label><label>Ime i prezime<input required value={newUser.full_name} onChange={e=>setNewUser({...newUser,full_name:e.target.value})}/></label><label>Lozinka (min. 10 znakova)<input required type="password" minLength={10} value={newUser.password} onChange={e=>setNewUser({...newUser,password:e.target.value})}/></label><label>Uloga<select value={newUser.role} onChange={e=>setNewUser({...newUser,role:e.target.value})}><option value="employee">Djelatnik</option><option value="manager">Voditelj odjela</option><option value="director">Direktor sektora</option><option value="admin">Administrator</option></select></label><label>Odjel<select value={newUser.department_id} onChange={e=>setNewUser({...newUser,department_id:e.target.value})}><option value="">Bez odjela</option>{depts.map(d=><option key={d.id} value={d.id}>{d.name}</option>)}</select></label><div className="actions"><button className="primary" disabled={loading}>Dodaj korisnika</button></div></form></section></>}
   <footer>ReportFlow · Interni sustav mjesečnih planova i izvješća</footer>
  </main>
 </div>
}
function previewNumbers(items:{id:string;parent_id:string|null}[],section:number):Map<string,string>{
 const ids=new Set(items.map(i=>i.id));const children=new Map<string|null,typeof items>();
 for(const item of items){const parent=item.parent_id&&ids.has(item.parent_id)?item.parent_id:null;const list=children.get(parent)||[];list.push(item);children.set(parent,list);}
 const numbers=new Map<string,string>();const walk=(parent:string|null,prefix:string)=>{(children.get(parent)||[]).forEach((item,index)=>{if(numbers.has(item.id))return;const n=`${prefix}.${index+1}`;numbers.set(item.id,n);walk(item.id,n);});};
 walk(null,String(section));items.filter(item=>!numbers.has(item.id)).forEach((item,index)=>{numbers.set(item.id,`${section}.${items.length+index+1}`);});return numbers;
}
function SectorEditor({doc,busy,onCancel,onSave}:{doc:SectorDoc;busy:boolean;onCancel:()=>void;onSave:(snapshot:SectorSnapshot,introduction:string,conclusion:string)=>void}){
 const [snapshot,setSnapshot]=useState<SectorSnapshot>(()=>JSON.parse(JSON.stringify(doc.snapshot)));
 const [intro,setIntro]=useState(doc.introduction);const [conclusion,setConclusion]=useState(doc.conclusion);
 const edit=(si:number,ii:number,key:'title'|'description'|'content',value:string)=>setSnapshot(prev=>({...prev,sections:prev.sections.map((s,index)=>index!==si?s:{...s,items:s.items.map((i,j)=>j===ii?{...i,[key]:value}:i)})}));
 const remove=(si:number,ii:number)=>setSnapshot(prev=>({...prev,sections:prev.sections.map((s,index)=>index!==si?s:{...s,items:s.items.filter((_,j)=>j!==ii)})}));
 const move=(si:number,ii:number,delta:number)=>setSnapshot(prev=>({...prev,sections:prev.sections.map((s,index)=>{if(index!==si)return s;const list=[...s.items];const item=list[ii];const siblings=list.map((x,j)=>({x,j})).filter(x=>x.x.parent_id===item.parent_id);const position=siblings.findIndex(x=>x.j===ii);const target=siblings[position+delta];if(!target)return s;[list[ii],list[target.j]]=[list[target.j],list[ii]];return {...s,items:list};})}));
 return <section className="panel editor-panel"><div className="panel-head"><div><h2>Uređivanje revizije {doc.revision}</h2><p>Uređuje se samo nacrt sektorskog dokumenta. Izvorna izvješća odjela ostaju nepromijenjena.</p></div><span className="chip amber">Nespremljene izmjene</span></div><label>Uvod<textarea rows={2} value={intro} onChange={e=>setIntro(e.target.value)}/></label>
 <div className="document-preview"><p><strong>Predmet: {doc.title}</strong></p>{snapshot.sections.map((sec,si)=><div key={sec.department_id}><h3>{si+1}. {sec.name.toUpperCase()}</h3>{sec.items.map((item,ii)=><div className="preview-item" key={item.id}><div className="row-actions"><strong>{previewNumbers(sec.items,si+1).get(item.id)}. {item.parent_id?'↳ ':''}{item.title}</strong><button className="outline compact" disabled={busy} title="Gore" onClick={()=>move(si,ii,-1)}><ArrowUp size={15}/></button><button className="outline compact" disabled={busy} title="Dolje" onClick={()=>move(si,ii,1)}><ArrowDown size={15}/></button><button className="outline compact" disabled={busy} onClick={()=>remove(si,ii)}>Ukloni</button></div><label>Naslov<input value={item.title} onChange={e=>edit(si,ii,'title',e.target.value)}/></label><label>Opis aktivnosti<textarea rows={2} value={item.description||''} onChange={e=>edit(si,ii,'description',e.target.value)}/></label><label>Mjesečni zapis<textarea rows={2} value={item.content||''} onChange={e=>edit(si,ii,'content',e.target.value)}/></label></div>)}</div>)}</div>
 <label>Zaključak<textarea rows={2} value={conclusion} onChange={e=>setConclusion(e.target.value)}/></label><div className="actions"><button className="outline" onClick={onCancel}>Odustani</button><button className="primary" disabled={busy} onClick={()=>onSave(snapshot,intro,conclusion)}><Save size={16}/> Spremi nacrt</button></div><p className="hint">Numeracija se ponovno računa u Word generatoru prema hijerarhiji stavki. Uklonjene stavke moguće je vratiti izradom nove revizije iz odjela.</p></section>
}
function Empty({text}:{text:string}){return <div className="empty"><FileText size={28}/><p>{text}</p></div>}
function Login({onLogin}:{onLogin:(t:string)=>void}){
 const [username,setUsername]=useState('admin'),[password,setPassword]=useState(''),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 const submit=async(e:React.FormEvent)=>{e.preventDefault();setBusy(true);setError('');try{const r=await post<{token:string}>('/auth/login',{username,password});onLogin(r.token);}catch(e){setError((e as Error).message);}finally{setBusy(false);}};
 return <div className="login-wrap"><div className="login-box"><div className="login-logo"><BookOpen size={27}/></div><h1>ReportFlow</h1><p>Mjesečni planovi i izvješća Sektora sigurnosti plovidbe</p><form onSubmit={submit}><label>Korisničko ime<input autoComplete="username" required value={username} onChange={e=>setUsername(e.target.value)}/></label><label>Lozinka<input type="password" autoComplete="current-password" required value={password} onChange={e=>setPassword(e.target.value)}/></label>{error&&<div className="alert danger">{error}</div>}<button className="primary" disabled={busy}>{busy?'Prijava...':'Prijavi se'}</button></form></div></div>
}

createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
