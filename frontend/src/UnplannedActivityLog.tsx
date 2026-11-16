import {useEffect,useState,type FormEvent} from 'react';
import {api,post,patch,q,type Activity,type UnplannedActivity} from './api';

type Props={departmentId:string;activities:Activity[];canWrite:boolean;currentUserId:string;canManage:boolean;onSaved?:()=>Promise<void>};
const localDate=()=>{
 const d=new Date();
 return d.getFullYear()+'-'+String(d.getMonth()+1).padStart(2,'0')+'-'+String(d.getDate()).padStart(2,'0');
};
const monthOf=(date:string)=>date.slice(0,7);
const dateLabel=(date:string)=>new Date(date+'T12:00:00').toLocaleDateString('hr-HR',{dateStyle:'medium'});

export default function UnplannedActivityLog({departmentId,activities,canWrite,currentUserId,canManage,onSaved}:Props){
 const [period,setPeriod]=useState(()=>monthOf(localDate()));
 const [performedOn,setPerformedOn]=useState(localDate());
 const [title,setTitle]=useState('');
 const [description,setDescription]=useState('');
 const [relatedActivityId,setRelatedActivityId]=useState('');
 const [items,setItems]=useState<UnplannedActivity[]>([]);
 const [editingId,setEditingId]=useState<string|null>(null);
 const [busy,setBusy]=useState(false);
 const [error,setError]=useState('');
 const [notice,setNotice]=useState('');

 useEffect(()=>{
  if(!departmentId){setItems([]);return;}
  let live=true;
  const [year,month]=period.split('-').map(Number);
  api<UnplannedActivity[]>('/unplanned-activities'+q({department_id:departmentId,year,month}))
   .then(rows=>{if(live)setItems(rows);})
   .catch(e=>{if(live)setError(e instanceof Error?e.message:String(e));});
  return ()=>{live=false;};
 },[departmentId,period]);

 const reset=()=>{
  setEditingId(null);setPerformedOn(localDate());setTitle('');setDescription('');setRelatedActivityId('');
 };
 const save=async(e:FormEvent)=>{
  e.preventDefault();setBusy(true);setError('');setNotice('');
  try{
   const payload={performed_on:performedOn,title:title.trim(),description:description.trim(),related_activity_id:relatedActivityId||null,...(!editingId?{department_id:departmentId}:{})};
   if(editingId)await patch('/unplanned-activities/'+editingId,payload);
   else await post('/unplanned-activities',payload);
   setPeriod(monthOf(performedOn));reset();setNotice('Obavljeni posao je evidentiran.');await onSaved?.();
  }catch(e){setError(e instanceof Error?e.message:String(e));}
  finally{setBusy(false);}
 };
 const edit=(item:UnplannedActivity)=>{
  setEditingId(item.id);setPerformedOn(item.performed_on);setTitle(item.title);setDescription(item.description);
  setRelatedActivityId(item.related_activity_id??'');setError('');setNotice('');
  window.scrollTo({top:0,behavior:'smooth'});
 };
 return <section className="panel rf-unplanned-panel">
  <div className="ux-intro">
   <div><h2>Evidentiraj obavljeni posao</h2>
    <p>Za intervencije i poslove koji nisu bili u mjesečnom planu. Ne dodaje se u trajni registar aktivnosti.</p></div>
   <span className="rf-unplanned-mark">Neplanirano</span>
  </div>
  {canWrite&&<form className="rf-unplanned-form" onSubmit={save}>
   <div className="rf-unplanned-fields">
    <label>Što je napravljeno? *
     <input required maxLength={250} value={title} onChange={e=>setTitle(e.target.value)} placeholder="Npr. Izvanredni pregled VHF uređaja"/>
    </label>
    <label>Opis obavljenog posla *
     <textarea required rows={3} maxLength={12000} value={description} onChange={e=>setDescription(e.target.value)} placeholder="Ukratko opiši što je napravljeno i kakav je rezultat."/>
    </label>
    <div className="rf-unplanned-meta">
     <label>Datum obavljanja
      <input required type="date" value={performedOn} onChange={e=>setPerformedOn(e.target.value)}/>
     </label>
     <label>Povezana planirana aktivnost <span className="rf-optional">(neobavezno)</span>
      <select value={relatedActivityId} onChange={e=>setRelatedActivityId(e.target.value)}>
       <option value="">Nije povezano</option>
       {activities.map(a=><option value={a.id} key={a.id}>{a.title}</option>)}
      </select>
     </label>
    </div>
   </div>
   {error&&<p className="alert danger" role="alert">{error}</p>}
   {notice&&<p className="rf-unplanned-notice" role="status">{notice}</p>}
   <div className="rf-unplanned-actions">
    <button className="primary" type="submit" disabled={busy||!title.trim()||!description.trim()||!performedOn}>{busy?'Spremanje…':editingId?'Spremi izmjene':'Spremi obavljeni posao'}</button>
    {editingId&&<button className="outline" type="button" disabled={busy} onClick={reset}>Odustani od izmjena</button>}
   </div>
  </form>}
  <div className="rf-unplanned-history">
   <div className="rf-unplanned-history-head"><div><h3>Evidentirani neplanirani poslovi</h3><p>{items.length} zapisa u odabranom mjesecu</p></div>
    <label>Mjesec <input type="month" value={period} onChange={e=>{if(e.target.value)setPeriod(e.target.value);}}/></label>
   </div>
   {items.length===0?<p className="rf-unplanned-empty">Nema evidentiranih neplaniranih poslova za ovaj mjesec.</p>:
    <div className="rf-unplanned-list">{items.map(item=><article key={item.id}>
     <div className="rf-unplanned-item-head"><div><time dateTime={item.performed_on}>{dateLabel(item.performed_on)}</time><h4>{item.title}</h4></div>
      {canWrite&&(canManage||item.created_by===currentUserId)&&<button type="button" className="outline compact" onClick={()=>edit(item)}>Uredi</button>}
     </div>
     <p>{item.description}</p>
     {item.related_activity_title&&<small>Povezano s aktivnošću: {item.related_activity_title}</small>}
     <small>{item.created_by_name??'Nepoznat autor'} · uneseno {new Date(item.created_at).toLocaleString('hr-HR',{dateStyle:'short',timeStyle:'short'})}</small>
    </article>)}</div>}
  </div>
 </section>;
}
