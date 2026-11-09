import {useEffect,useState} from 'react';
import {Check,ChevronDown,ChevronRight,Save} from 'lucide-react';
import type {Activity,Update} from './api';

type Kind='plan'|'report';
type Props={
 item:Activity;initial?:Update;planReference?:string;kind:Kind;disabled:boolean;busy:boolean;
 onSave:(content:string,progress:string,include:boolean)=>void;
};
const labels:Record<string,string>={in_progress:'U tijeku',completed:'Završeno',postponed:'Odgođeno',not_started:'Nije započeto'};
const typeName:Record<string,string>={ongoing:'Trajna aktivnost',project:'Projekt',one_off:'Jednokratna'};
export default function MonthlyEntry({item,initial,planReference,kind,disabled,busy,onSave}:Props){
 const [content,setContent]=useState(initial?.content??'');
 const [progress,setProgress]=useState(initial?.progress??(kind==='plan'?'not_started':'in_progress'));
 const [include,setInclude]=useState(initial?.include_in_document??true);
 const [expanded,setExpanded]=useState(Boolean(initial?.content));
 useEffect(()=>{
  setContent(initial?.content??'');
  setProgress(initial?.progress??(kind==='plan'?'not_started':'in_progress'));
  setInclude(initial?.include_in_document??true);
 },[initial?.id,initial?.content,initial?.progress,initial?.include_in_document,kind,item.id]);
 const changed=content!==(initial?.content??'')||include!==(initial?.include_in_document??true)||
  (kind==='report'&&progress!==(initial?.progress??'in_progress'));
 const saved=Boolean(initial);
 const placeholder=kind==='plan'
  ?'Primjer: Izvršiti pregled VHF opreme na tri lokacije, provjeriti radne kanale i izraditi zapisnik.'
  :'Primjer: Izvršen pregled na dvije od tri lokacije. Preostala lokacija odgođena zbog vremenskih uvjeta.';
 const description=kind==='plan'
  ?'Napiši što namjeravaš obaviti tijekom odabranog mjeseca.'
  :'Napiši što je stvarno napravljeno, što nije dovršeno i eventualne razloge odstupanja od plana.';
 return <article className={'monthly-card'+(expanded?' is-open':'')}>
  <div className="monthly-card-top">
   <button className="monthly-expand" type="button" onClick={()=>setExpanded(p=>!p)} aria-expanded={expanded}>
    {expanded?<ChevronDown size={18}/>:<ChevronRight size={18}/>}
    <span className="monthly-card-heading"><strong>{item.title}</strong><small>{typeName[item.type]??item.type}</small></span>
   </button>
   <div className="monthly-card-flags">{changed&&<span className="monthly-unsaved">Nespremljene izmjene</span>}
    <span className={'chip '+(saved?'green':'')}>{saved?'Spremljeno':'Nije uneseno'}</span>
   </div>
  </div>
  {expanded&&<div className="monthly-card-body">
   {item.description.trim()&&<div className="monthly-reference"><span>Opis aktivnosti iz registra</span><p>{item.description}</p></div>}
   {kind==='report'&&<div className="monthly-plan-ref"><strong>Plan za ovaj mjesec</strong>
    <p>{planReference?.trim()||'Nema prethodno unesenog plana za ovu aktivnost. Izvješće se može unijeti i bez plana.'}</p>
   </div>}
   <label className="monthly-field" htmlFor={'entry-'+item.id}>
    <strong>{kind==='plan'?'Što planiramo napraviti?':'Što je stvarno realizirano?'}</strong>
    <span>{description}</span>
   </label>
   <textarea id={'entry-'+item.id} disabled={disabled} rows={4} value={content} onChange={e=>setContent(e.target.value)} placeholder={placeholder}/>
   <div className="monthly-controls">
    {kind==='report'&&<label className="monthly-progress">Status realizacije
     <select value={progress} disabled={disabled} onChange={e=>setProgress(e.target.value)}>
      {Object.entries(labels).map(([value,label])=><option value={value} key={value}>{label}</option>)}
     </select>
    </label>}
    <label className="monthly-include"><input type="checkbox" disabled={disabled} checked={include} onChange={e=>setInclude(e.target.checked)}/>
     Uključi ovu aktivnost u {kind==='plan'?'mjesečni plan':'mjesečno izvješće'}
    </label>
    <button type="button" className="primary monthly-save" disabled={disabled||busy||!changed} onClick={()=>onSave(content,progress,include)}>
     {changed?<Save size={16}/>:<Check size={16}/>} {saved?'Spremi promjene':'Spremi unos'}
    </button>
   </div>
   {disabled&&<p className="monthly-readonly">Uređivanje trenutno nije dostupno za ovu aktivnost ili je dokument već predan.</p>}
  </div>}
 </article>;
}
