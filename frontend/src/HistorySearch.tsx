import React,{useEffect,useState} from 'react';
import {api,type Dept,type User} from './api';

type SearchRow={
 activity_id:string;department_id:string;department_name:string;
 title:string;description:string;status:string;
 year:number|null;month:number|null;kind:string|null;
 content:string|null;progress:string|null;updated_at:string|null;
 include_in_document:boolean|null;
};
type HistoryEntry={
 id:string;year:number;month:number;kind:string;content:string;
 progress:string;updated_at:string;updated_by_name:string|null;created_at:string;created_by_name:string|null;creation_time_estimated:boolean;
 include_in_document:boolean;
};
type HistoryData={
 activity:{id:string;title:string;description:string;department_name:string;status:string;created_at:string;updated_at:string;created_by_name:string|null};
 entries:HistoryEntry[];
};
const statuses:Record<string,string>={in_progress:'U tijeku',completed:'Završeno',postponed:'Odgođeno',not_started:'Nije započeto',paused:'Pauzirano',cancelled:'Otkazano'};
const kindLabel=(k:string)=>k==='plan'?'Plan':'Izvješće';
const monthLabel=(month:number,year:number)=>new Date(year,month-1,1).toLocaleDateString('hr-HR',{month:'long',year:'numeric'});

export default function HistorySearch({user,depts,initialActivityId}:{user:User;depts:Dept[];initialActivityId:string|null}){
 const sector=user.role==='director';
 const [query,setQuery]=useState('');
 const [departmentId,setDepartmentId]=useState(sector?'':user.department_id??'');
 const [year,setYear]=useState('');
 const [month,setMonth]=useState('');
 const [kind,setKind]=useState('');
 const [results,setResults]=useState<SearchRow[]>([]);
 const [history,setHistory]=useState<HistoryData|null>(null);
 const [error,setError]=useState('');
 const [busy,setBusy]=useState(false);
 const [searched,setSearched]=useState(false);
 async function openHistory(id:string){
  setBusy(true);setError('');
  try {setHistory(await api<HistoryData>('/activities/'+encodeURIComponent(id)+'/history'));}
  catch(e){setError((e as Error).message);}
  finally{setBusy(false);}
 }
 useEffect(()=>{if(initialActivityId)void openHistory(initialActivityId);},[initialActivityId]);
 async function search(e:React.FormEvent){
  e.preventDefault();setBusy(true);setError('');
  try{
   const params=new URLSearchParams({q:query.trim(),limit:'100'});
   if(departmentId)params.set('department_id',departmentId);
   if(year)params.set('year',year);
   if(month)params.set('month',month);
   if(kind)params.set('kind',kind);
   setResults(await api<SearchRow[]>('/activity-search?'+params.toString()));
   setSearched(true);
  }catch(e){setError((e as Error).message);}
  finally{setBusy(false);}
 }
 return <div className="history-search-layout">
  <section className="panel">
   <h2>Pretraživanje aktivnosti</h2>
   <p>Pretraži nazive, opise aktivnosti i mjesečne zapise. Pretraživanje nije ograničeno na tekući mjesec.</p>
   <form className="form-grid" onSubmit={search}>
    <label className="wide">Pojam (najmanje 2 znaka)
     <input required minLength={2} maxLength={120} value={query} onChange={e=>setQuery(e.target.value)} placeholder="Npr. CYSCROMS, Jadrija, HAKOM..."/>
    </label>
    {sector&&<label>Odjel
     <select value={departmentId} onChange={e=>setDepartmentId(e.target.value)}>
      <option value="">Svi odjeli</option>
      {depts.map(d=><option value={d.id} key={d.id}>{d.name}</option>)}
     </select>
    </label>}
    <label>Godina (neobavezno)
     <input type="number" min="2000" max="2100" value={year} onChange={e=>setYear(e.target.value)} placeholder="Sve godine"/>
    </label>
    <label>Mjesec
     <select value={month} onChange={e=>setMonth(e.target.value)}>
      <option value="">Svi mjeseci</option>
      {Array.from({length:12},(_,i)=><option key={i} value={i+1}>{new Date(2026,i,1).toLocaleDateString('hr-HR',{month:'long'})}</option>)}
     </select>
    </label>
    <label>Vrsta dokumenta
     <select value={kind} onChange={e=>setKind(e.target.value)}>
      <option value="">Plan i izvješće</option><option value="plan">Plan</option><option value="report">Izvješće</option>
     </select>
    </label>
    <div className="actions"><button className="primary" disabled={busy||query.trim().length<2}>Pretraži</button></div>
   </form>
   {error&&<div className="alert danger" role="alert">{error}</div>}
   {searched&&<p role="status">Pronađeno zapisa: {results.length}{results.length===100?' (prikazano prvih 100)':''}</p>}
   {results.map((r,i)=><div className="activity-row" key={r.activity_id+'-'+String(r.year)+'-'+String(r.month)+'-'+String(r.kind)+'-'+i}>
     <div>
      <strong>{r.title}</strong>
      <p>{r.department_name}{r.year&&r.month?' · '+monthLabel(r.month,r.year)+' · '+kindLabel(r.kind??'report'):''}</p>
      {r.description&&<p>{r.description}</p>}
      {r.content&&<p><strong>Mjesečni zapis:</strong> {r.content}</p>}
     </div>
     <button className="outline" type="button" disabled={busy} onClick={()=>void openHistory(r.activity_id)}>Povijest</button>
    </div>)}
  </section>
  {history&&<section className="panel" aria-label="Povijest aktivnosti">
   <div className="panel-head"><div><h2>Povijest: {history.activity.title}</h2><p>{history.activity.department_name}</p></div>
    <button type="button" className="outline" onClick={()=>setHistory(null)}>Zatvori</button></div>
   {history.activity.description&&<p><strong>Opis iz registra:</strong> {history.activity.description}</p>}
   <p><strong>Trenutni status:</strong> {statuses[history.activity.status]??history.activity.status}</p>
   <p className="rf-history-date">Kreirano: {new Date(history.activity.created_at).toLocaleString('hr-HR',{dateStyle:'short',timeStyle:'medium'})}{history.activity.created_by_name?' · '+history.activity.created_by_name:''} · Zadnja izmjena: {new Date(history.activity.updated_at).toLocaleString('hr-HR',{dateStyle:'short',timeStyle:'medium'})}</p>
   {history.entries.length===0?<p>Nema spremljenih mjesečnih zapisa. Aktivnost postoji samo u registru.</p>:
    <div className="history-timeline">{history.entries.map(entry=><article key={entry.id} className="history-entry">
     <div><strong>{monthLabel(entry.month,entry.year)}</strong> · {kindLabel(entry.kind)}
      <span className="chip">{statuses[entry.progress]??entry.progress}</span></div>
     <p>{entry.content||'Nije unesen opis za ovaj mjesec.'}</p>
     <small>Kreirano: {new Date(entry.created_at).toLocaleString('hr-HR',{dateStyle:'short',timeStyle:'medium'})}{entry.created_by_name?' · '+entry.created_by_name:''}{entry.creation_time_estimated?' (povijesna procjena)':''}
       <br/>Izmijenjeno: {new Date(entry.updated_at).toLocaleString('hr-HR',{dateStyle:'short',timeStyle:'medium'})}{entry.updated_by_name?' · '+entry.updated_by_name:''}
       {!entry.include_in_document?' · Nije uključeno u dokument':''}</small>
    </article>)}</div>}
  </section>}
 </div>;
}
