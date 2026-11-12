import {useCallback,useEffect,useState} from 'react';
import {api,post,q} from './api';
type Candidate={id:string;title:string;progress:string;previous_report:string;already_planned:boolean};
type Response={source_year:number;source_month:number;items:Candidate[]};
const progressLabels:Record<string,string>={in_progress:'U tijeku',postponed:'Odgođeno',not_started:'Nije započeto'};
export default function CarryPlanner({departmentId,year,month,canCarry,onApplied}:{departmentId:string;year:number;month:number;canCarry:boolean;onApplied:()=>Promise<void>}){
 const [open,setOpen]=useState(false),[data,setData]=useState<Response|null>(null);
 const [chosen,setChosen]=useState<string[]>([]),[busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState('');
 const load=useCallback(async()=>{
  setBusy(true);setError('');
  try{const result=await api<Response>('/updates/carry-candidates'+q({department_id:departmentId,year,month}));setData(result);setChosen(result.items.filter(x=>!x.already_planned).map(x=>x.id));}
  catch(e){setError(e instanceof Error?e.message:String(e));}
  finally{setBusy(false);}
 },[departmentId,year,month]);
 useEffect(()=>{setOpen(false);setData(null);setChosen([]);setError('');setNotice('');},[departmentId,year,month]);
 const apply=async()=>{
  setBusy(true);setError('');
  try{
   const result=await post<{created:number}>('/updates/carry-forward',{department_id:departmentId,year,month,kind:'plan',activity_ids:chosen});
   setNotice('Preneseno '+result.created+' aktivnosti u plan.');await onApplied();await load();setOpen(false);
  }catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}
 };
 return <div className="carry-planner">
  <button type="button" className="outline" disabled={!canCarry||busy} onClick={()=>{const next=!open;setOpen(next);if(next)void load();}}>Prenesi nedovršene iz prošlog izvješća</button>
  {notice&&<p role="status" className="carry-notice">{notice}</p>}
  {error&&<p role="alert" className="alert danger">{error}</p>}
  {open&&<div className="carry-panel">
   <h3>Odaberi aktivnosti za prijenos</h3>
   <p>Iz izvješća za {data?String(data.source_month).padStart(2,'0')+'/'+data.source_year:'prethodni mjesec'} u plan za {String(month).padStart(2,'0')+'/'+year}. Tekst prethodne realizacije neće se prepisivati.</p>
   {!data?<p>Učitavanje…</p>:data.items.length===0?<p>U prethodnom izvješću nema nedovršenih aktivnosti.</p>:
    <div className="carry-options">{data.items.map(item=><label key={item.id} className={item.already_planned?'carry-existing':''}>
     <input type="checkbox" checked={chosen.includes(item.id)} disabled={busy||item.already_planned} onChange={e=>setChosen(ids=>e.target.checked?[...ids,item.id]:ids.filter(x=>x!==item.id))}/>
     <span><strong>{item.title}</strong><small>{item.already_planned?'Već postoji u planu':progressLabels[item.progress]||item.progress}</small></span>
    </label>)}</div>}
   <div className="carry-actions"><button className="primary" disabled={busy||chosen.length===0} onClick={()=>void apply()}>Prenesi odabrane ({chosen.length})</button><button className="outline" onClick={()=>setOpen(false)}>Odustani</button></div>
  </div>}
 </div>;
}
