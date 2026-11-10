import {useCallback,useEffect,useState} from 'react';
import {api,post,patch} from './api';
type Person={id:string;name:string;role:string};
type Thread={id:string;title:string;status:'open'|'done';created_name:string;created_at:string;updated_at:string;message_count:number};
type Message={id:string;body:string;author:string;created_at:string};
type Event={event_type:string;author:string;created_at:string;payload:Record<string,string>};
type Detail=Thread&{participants:Person[];messages:Message[];events:Event[]};
const stamp=(v:string)=>new Date(v).toLocaleString('hr-HR',{dateStyle:'medium',timeStyle:'short'});
export default function ActivityConversation({activityId,canManage,onClose,openThread}:{activityId:string;canManage:boolean;onClose:()=>void;openThread?:string|null}){
 const [threads,setThreads]=useState<Thread[]>([]),[people,setPeople]=useState<Person[]>([]);
 const [selected,setSelected]=useState<string|null>(openThread??null),[detail,setDetail]=useState<Detail|null>(null);
 const [creating,setCreating]=useState(false),[title,setTitle]=useState(''),[initial,setInitial]=useState('');
 const [chosen,setChosen]=useState<string[]>([]),[addPerson,setAddPerson]=useState(''),[message,setMessage]=useState('');
 const [busy,setBusy]=useState(false),[error,setError]=useState('');
 const reload=useCallback(async()=>{try{const [ts,ps]=await Promise.all([api<Thread[]>('/activities/'+activityId+'/threads'),api<Person[]>('/activities/'+activityId+'/people')]);setThreads(ts);setPeople(ps);}catch(e){setError(e instanceof Error?e.message:String(e));}},[activityId]);
 const detailLoad=useCallback(async(id:string)=>{try{setDetail(await api<Detail>('/threads/'+id));await post('/threads/'+id+'/read',{});}catch(e){setError(e instanceof Error?e.message:String(e));}},[]);
 useEffect(()=>{setSelected(openThread??null);void reload();},[activityId,openThread,reload]);
 useEffect(()=>{if(selected)void detailLoad(selected);else setDetail(null);},[selected,detailLoad]);
 const perform=async(action:()=>Promise<unknown>)=>{setBusy(true);setError('');try{await action();await reload();if(selected)await detailLoad(selected);}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}};
 return <section className="collab-panel">
  <div className="collab-top"><h3>Chat / Rasprava</h3><button type="button" className="outline compact" onClick={onClose}>Zatvori</button></div>
  {error&&<div role="alert" className="alert danger">{error}</div>}
  {!selected&&<>
   {canManage&&<button className="primary" onClick={()=>setCreating(v=>!v)}>+ Pokreni raspravu</button>}
   {creating&&<form className="collab-form" onSubmit={e=>{e.preventDefault();void perform(async()=>{
    const result=await post<{id:string}>('/activities/'+activityId+'/threads',{title,participants:chosen,message:initial});
    setTitle('');setInitial('');setChosen([]);setCreating(false);setSelected(result.id);
   });}}>
    <label>Naslov rasprave<input required maxLength={250} value={title} onChange={e=>setTitle(e.target.value)} placeholder="Npr. Dovršiti mjesečni pregled"/></label>
    <label>Prva poruka<textarea rows={3} value={initial} onChange={e=>setInitial(e.target.value)} placeholder="Napiši što treba napraviti..."/></label>
    <fieldset className="collab-users"><legend>Dodaj sudionike</legend>{people.map(p=><label key={p.id}><input type="checkbox" checked={chosen.includes(p.id)} onChange={e=>setChosen(v=>e.target.checked?[...v,p.id]:v.filter(x=>x!==p.id))}/>{p.name}</label>)}</fieldset>
    <button className="primary" disabled={busy||!title.trim()}>Pokreni raspravu</button>
   </form>}
   <div className="collab-list">{threads.length?threads.map(t=><button key={t.id} onClick={()=>setSelected(t.id)}>
    <strong>{t.title}</strong><span>{t.status==='done'?'Dovršeno':'Otvoreno'} · {t.message_count} poruka · {stamp(t.updated_at)}</span>
   </button>):<p>Uz ovu aktivnost još nema rasprava.</p>}</div>
  </>}
  {selected&&<>
   <button className="collab-back" onClick={()=>setSelected(null)}>← Sve rasprave</button>
   {detail&&<><div className="collab-topic"><h4>{detail.title}</h4><span>{detail.status==='done'?'Dovršeno':'Otvoreno'} · otvoreno {stamp(detail.created_at)}</span></div>
    <div className="collab-members"><strong>Sudionici:</strong> {detail.participants.map(p=>p.name).join(', ')||'—'}</div>
    {canManage&&<div className="collab-manage">
     <select aria-label="Dodaj korisnika" value={addPerson} onChange={e=>setAddPerson(e.target.value)}><option value="">Dodaj korisnika…</option>{people.filter(p=>!detail.participants.some(existing=>existing.id===p.id)).map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select>
     <button className="outline" disabled={!addPerson||busy} onClick={()=>void perform(async()=>{await post('/threads/'+selected+'/participants',{user_id:addPerson});setAddPerson('');})}>Dodaj</button>
     <button className="outline" disabled={busy} onClick={()=>void perform(()=>patch('/threads/'+selected+'/status',{status:detail.status==='open'?'done':'open'}))}>{detail.status==='open'?'Označi dovršenim':'Ponovno otvori'}</button>
    </div>}
    <div className="collab-messages">{detail.messages.map(m=><article key={m.id}><div><strong>{m.author}</strong><time dateTime={m.created_at}>{stamp(m.created_at)}</time></div><p>{m.body}</p></article>)}{!detail.messages.length&&<p>Još nema poruka.</p>}</div>
    {detail.status==='open'&&<form className="collab-reply" onSubmit={e=>{e.preventDefault();void perform(async()=>{await post('/threads/'+selected+'/messages',{body:message});setMessage('');});}}><label>Nova poruka<textarea value={message} onChange={e=>setMessage(e.target.value)} rows={3} placeholder="Napiši komentar ili uputu..."/></label><button className="primary" disabled={busy||!message.trim()}>Pošalji</button></form>}
    <details className="collab-history"><summary>Povijest rasprave</summary>{detail.events.map((ev,i)=><p key={i}>{stamp(ev.created_at)} · {ev.author} · {ev.event_type==='created'?'Pokrenuta rasprava':ev.event_type==='participant_added'?'Dodan sudionik':'Promijenjen status'}</p>)}</details>
   </>}
  </>}
 </section>;
}
