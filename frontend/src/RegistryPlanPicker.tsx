import {useMemo,useState} from 'react';
import {Plus,Search,X,Check} from 'lucide-react';
import type {Activity} from './api';

type Props={activities:Activity[];plannedIds:string[];disabled:boolean;onAdd:(ids:string[])=>Promise<number>};
type Row={item:Activity;depth:number;matched:boolean;alreadyPlanned:boolean};
function treeOrder(items:Activity[]):Activity[]{
 const ids=new Set(items.map(item=>item.id));const result:Activity[]=[];const seen=new Set<string>();
 const visit=(parent:string|null)=>{
  items.filter(item=>(ids.has(item.parent_id||'')?item.parent_id:null)===parent).sort((a,b)=>a.sort_order-b.sort_order).forEach(item=>{if(seen.has(item.id))return;seen.add(item.id);result.push(item);visit(item.id);});
 };
 visit(null);items.forEach(item=>{if(!seen.has(item.id))result.push(item);});return result;
}
export default function RegistryPlanPicker({activities,plannedIds,disabled,onAdd}:Props){
 const [open,setOpen]=useState(false),[query,setQuery]=useState(''),[selected,setSelected]=useState<string[]>([]);
 const [limit,setLimit]=useState(30),[busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState('');
 const planned=new Set(plannedIds);
 const rows=useMemo<Row[]>(()=>{
  const normalized=query.trim().toLocaleLowerCase('hr');if(normalized.length<2)return [];
  const ordered=treeOrder(activities);
  const matches=ordered.filter(item=>!['completed','cancelled'].includes(item.status)&&(item.title+' '+item.description).toLocaleLowerCase('hr').includes(normalized));
  const available=matches.filter(item=>!planned.has(item.id));
  const shown=available.slice(0,limit);const matchedIds=new Set(shown.map(item=>item.id));
  const displayIds=new Set(shown.map(item=>item.id));const byId=new Map(activities.map(item=>[item.id,item]));
  for(const item of shown){let parent=item.parent_id,remaining=activities.length;while(parent&&remaining-->0){displayIds.add(parent);parent=byId.get(parent)?.parent_id??null;}}
  const depthOf=(item:Activity)=>{let depth=0,parent=item.parent_id,remaining=activities.length;while(parent&&remaining-->0){depth++;parent=byId.get(parent)?.parent_id??null;}return Math.min(depth,5);};
  return ordered.filter(item=>displayIds.has(item.id)).map(item=>({item,depth:depthOf(item),matched:matchedIds.has(item.id),alreadyPlanned:planned.has(item.id)}));
 },[activities,plannedIds,query,limit]);
 const normalized=query.trim().toLocaleLowerCase('hr');
 const totalMatches=normalized.length>=2?activities.filter(item=>!['completed','cancelled'].includes(item.status)&&(item.title+' '+item.description).toLocaleLowerCase('hr').includes(normalized)).length:0;
 const availableMatches=normalized.length>=2?activities.filter(item=>!['completed','cancelled'].includes(item.status)&&!planned.has(item.id)&&(item.title+' '+item.description).toLocaleLowerCase('hr').includes(normalized)).length:0;
 const openPicker=()=>{setQuery('');setSelected([]);setLimit(30);setError('');setNotice('');setOpen(true);};
 const add=async()=>{if(!selected.length)return;setBusy(true);setError('');try{const created=await onAdd(selected);setNotice('Dodano '+created+' aktivnosti u plan.');setOpen(false);setSelected([]);setQuery('');}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}};
 return <div className="rf-plan-picker">
  <button type="button" className="primary" disabled={disabled||activities.length===0} onClick={openPicker}><Plus size={16}/> Dodaj iz registra</button>
  {notice&&<span className="rf-plan-picker-notice" role="status">{notice}</span>}
  {open&&<div className="rf-picker-backdrop" role="presentation">
   <section className="rf-picker-dialog" role="dialog" aria-modal="true" aria-labelledby="rf-picker-title">
    <header className="rf-picker-header"><div><h3 id="rf-picker-title">Dodaj aktivnosti iz registra</h3><p>Odaberi samo aktivnosti koje želiš uvrstiti u mjesečni plan.</p></div><button type="button" className="icon-button" aria-label="Zatvori" onClick={()=>setOpen(false)}><X size={19}/></button></header>
    <label className="rf-picker-search"><Search size={17}/><input autoFocus value={query} onChange={e=>{setQuery(e.target.value);setLimit(30);}} placeholder="Upiši naziv ili dio opisa aktivnosti (najmanje 2 znaka)"/></label>
    <div className="rf-picker-results">
     {normalized.length<2?<p className="rf-picker-hint">Registar se ne prikazuje cijeli. Upiši barem dva znaka za pretraživanje.</p>:rows.length===0?<p className="rf-picker-hint">{totalMatches>0&&availableMatches===0?'Sve pronađene aktivnosti već su u planu.':'Nema aktivnosti koje odgovaraju pretrazi.'}</p>:
      <>{rows.map(({item,depth,matched,alreadyPlanned})=><label key={item.id} className={'rf-picker-row'+(!matched?' is-context':'')+(alreadyPlanned?' is-planned':'')} style={{paddingLeft:12+depth*18}}>
       {matched&&!alreadyPlanned?<input type="checkbox" checked={selected.includes(item.id)} disabled={busy} onChange={e=>setSelected(ids=>e.target.checked?[...ids,item.id]:ids.filter(id=>id!==item.id))}/>:<span className="rf-picker-checkbox-placeholder"/>}
       <span className="rf-picker-row-text"><strong>{item.title}</strong>{item.description&&<small>{item.description}</small>}</span>{alreadyPlanned&&matched&&<em>Već u planu</em>}{!matched&&<em>Nadređena aktivnost</em>}
      </label>)}
      {availableMatches>limit&&<button type="button" className="outline rf-picker-more" onClick={()=>setLimit(n=>n+30)}>Prikaži još rezultata ({availableMatches-limit} preostalih)</button>}</>}
    </div>
    <footer className="rf-picker-footer"><span>{selected.length} odabrano</span><button type="button" className="outline" onClick={()=>setOpen(false)} disabled={busy}>Odustani</button><button type="button" className="primary" onClick={()=>void add()} disabled={busy||selected.length===0}><Check size={16}/> Dodaj odabrane</button></footer>
    {error&&<p className="rf-picker-error" role="alert">{error}</p>}
   </section>
  </div>}
 </div>;
}