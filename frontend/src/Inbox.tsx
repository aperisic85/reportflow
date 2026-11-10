import {useCallback,useEffect,useState} from 'react';
import {api} from './api';
export type InboxItem={id:string;title:string;status:string;activity_id:string;activity_title:string;department_name:string;unread:number;updated_at:string;last_message_at:string|null};
const stamp=(v:string)=>new Date(v).toLocaleString('hr-HR',{dateStyle:'medium',timeStyle:'short'});
export default function Inbox({onOpen}:{onOpen:(activityId:string,threadId:string)=>void}){
 const [items,setItems]=useState<InboxItem[]>([]),[error,setError]=useState('');
 const refresh=useCallback(async()=>{try{setItems(await api<InboxItem[]>('/inbox'));setError('');}catch(e){setError(e instanceof Error?e.message:String(e));}},[]);
 useEffect(()=>{void refresh();},[refresh]);
 return <section className="panel inbox-page">
  <div className="collab-top"><div><h2>Inbox</h2><p>Rasprave u kojima sudjeluješ.</p></div><button className="outline" onClick={()=>void refresh()}>Osvježi</button></div>
  {error&&<div className="alert danger">{error}</div>}
  {!items.length?<p>Nema poruka u inboxu.</p>:<div className="inbox-list">{items.map(item=><button key={item.id} type="button" onClick={()=>onOpen(item.activity_id,item.id)} className={item.unread>0?'inbox-unread':''}>
    <span><strong>{item.title}</strong><small>{item.activity_title} · {item.department_name}</small></span>
    <span className="inbox-meta">{item.unread>0?<b>{item.unread} novo</b>:<small>Pročitano</small>}<time>{stamp(item.updated_at)}</time></span>
   </button>)}</div>}
 </section>;
}
