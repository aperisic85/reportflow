import type {CSSProperties} from 'react';
import type {SectorDoc,SectorSnapshot} from './api';
import {numberedActivities} from './activityHierarchy';
type DepartmentVersion={id:string;version:number;snapshot:{items:Array<{id:string;title:string;description:string;content:string;progress:string;parent_id:string|null}>;kind:string;year:number;month:number};author:string;submitted_at:string};
export type {DepartmentVersion};
const stamp=(v:string)=>new Date(v).toLocaleString('hr-HR',{dateStyle:'medium',timeStyle:'short'});
const progressLabels:Record<string,string>={in_progress:'U tijeku',completed:'Završeno',postponed:'Odgođeno',not_started:'Nije započeto'};
function Items({items,kind}:{items:Array<{id:string;title:string;description?:string;content?:string;progress?:string}>;kind:'plan'|'report'}){
 return <div className="report-items">{numberedActivities(items).map(({item,number,depth})=><article key={item.id} className={'report-item'+(depth>0?' report-item-child':'')} style={{marginLeft:Math.min(depth,5)*24} as CSSProperties}>
  <h4><span className="report-item-number">{number}</span> {item.title}</h4>
  {item.description?.trim()&&<p>{item.description}</p>}
  {item.content?.trim()&&<p className="report-actual"><strong>{kind==='report'?'Mjesečna realizacija:':'Mjesečni plan:'}</strong> {item.content}</p>}
  {item.progress&&<small>{progressLabels[item.progress]??item.progress}</small>}
 </article>)}</div>;
}
export function SectorReportPreview({doc,onClose}:{doc:SectorDoc;onClose:()=>void}){
 const snap=doc.snapshot as SectorSnapshot;
 return <section className="report-reader panel">
  <div className="collab-top"><div><span className="report-label">Digitalni pregled</span><h2>{doc.title}</h2>
   <p>Revizija {doc.revision} · {doc.status==='locked'?'Zaključano':'Nacrt'} · kreirano {stamp(doc.created_at)}{doc.created_by_name?' · '+doc.created_by_name:''}</p></div>
   <button className="outline" type="button" onClick={onClose}>Zatvori</button></div>
  {doc.introduction.trim()&&<div className="report-intro"><h3>Uvod</h3><p>{doc.introduction}</p></div>}
  {snap.sections.map((sec,index)=><section className="report-section" key={sec.department_id}>
   <h3>{index+1}. {sec.name}</h3>
   <p className="rf-preview-source">{sec.source_version!=null?'Izvor: predana verzija v'+sec.source_version:'Izvorna verzija nije evidentirana'}{sec.source_submitted_at?' · '+stamp(sec.source_submitted_at):''}{sec.source_submitted_by?' · '+sec.source_submitted_by:''}</p>
   <Items items={sec.items} kind={snap.kind as 'plan'|'report'}/>
  </section>)}
  {doc.conclusion.trim()&&<div className="report-intro"><h3>Zaključak</h3><p>{doc.conclusion}</p></div>}
  <p className="report-footnote">Ovaj pregled prikazuje sadržaj spremljene revizije. Word/PDF izvoz ostaje zaseban službeni obrazac.</p>
 </section>;
}
export function DepartmentReportPreview({version,onClose}:{version:DepartmentVersion;onClose:()=>void}){
 const s=version.snapshot;
 return <section className="report-reader panel">
  <div className="collab-top"><div><span className="report-label">Digitalni pregled predane verzije</span>
   <h2>{s.kind==='plan'?'Mjesečni plan':'Mjesečno izvješće'} · {String(s.month).padStart(2,'0')}/{s.year}</h2>
   <p>Verzija {version.version} · {version.author} · {stamp(version.submitted_at)}</p></div>
   <button className="outline" type="button" onClick={onClose}>Zatvori</button></div>
  <Items items={s.items} kind={s.kind as 'plan'|'report'}/>
 </section>;
}
