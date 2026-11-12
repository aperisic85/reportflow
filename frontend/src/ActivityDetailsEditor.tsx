import {useEffect,useState} from 'react';
import {api,patch} from './api';
import type {Activity} from './api';
type Change={id:number;old_title:string;new_title:string;old_description:string;new_description:string;changed_at:string;author:string};
export default function ActivityDetailsEditor({activity,onSaved,onCancel}:{activity:Activity;onSaved:()=>Promise<void>;onCancel:()=>void}){
 const [title,setTitle]=useState(activity.title),[description,setDescription]=useState(activity.description);
 const [changes,setChanges]=useState<Change[]>([]),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 useEffect(()=>{setTitle(activity.title);setDescription(activity.description);void api<Change[]>('/activities/'+activity.id+'/field-history').then(setChanges).catch(()=>{});},[activity.id,activity.title,activity.description]);
 const save=async()=>{setBusy(true);setError('');try{
  await patch('/activities/'+activity.id+'/edit-details',{title,description});
  await onSaved();onCancel();
 }catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}};
 return <div className="activity-edit-panel">
  <h4>Uredi aktivnost</h4>
  <label>Naziv aktivnosti<input value={title} onChange={e=>setTitle(e.target.value)} maxLength={1000}/></label>
  <label>Opis aktivnosti<textarea rows={3} value={description} onChange={e=>setDescription(e.target.value)} maxLength={12000}/></label>
  {error&&<p className="alert danger" role="alert">{error}</p>}
  <div className="activity-edit-actions"><button type="button" className="primary" disabled={busy||!title.trim()||(title===activity.title&&description===activity.description)} onClick={()=>void save()}>Spremi izmjene</button><button type="button" className="outline" onClick={onCancel}>Odustani</button></div>
  <details><summary>Povijest promjena naziva i opisa ({changes.length})</summary>
   {changes.length===0?<p>Još nema evidentiranih promjena.</p>:changes.map(c=><p key={c.id}><time dateTime={c.changed_at}>{new Date(c.changed_at).toLocaleString('hr-HR')}</time> · {c.author}<strong> {c.old_title!==c.new_title?c.old_title+' → '+c.new_title:'Promjena opisa'}</strong></p>)}
  </details>
 </div>;
}
