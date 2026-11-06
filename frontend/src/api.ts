export type User = {id:string;username:string;name:string;role:'admin'|'director'|'manager'|'employee';department_id:string|null};
export type Dept = {id:string;code:string;name:string;sort_order:number};
export type Activity = {id:string;department_id:string;parent_id:string|null;title:string;description:string;type:string;status:string;sort_order:number;created_by:string|null};
export type Update = {id:string;activity_id:string;content:string;progress:string;include_in_document:boolean};
export type SectorDoc = {id:string;title:string;status:string;revision:number;created_at:string;snapshot:{sections:{name:string;items:{id:string;title:string;content:string;parent_id:string|null}[]}[]};introduction:string;conclusion:string};
const token = () => sessionStorage.getItem('rf_token');
export async function api<T>(path:string, options:RequestInit={}):Promise<T> {
  const res=await fetch('/api'+path,{...options,headers:{...(options.body?{'Content-Type':'application/json'}:{}),...(token()?{Authorization:'Bearer '+token()}:{}),...options.headers}});
  if (!res.ok) {let error=res.statusText;try {const data=await res.json();error=data.error??error;}catch { /* non-JSON error */ }throw new Error(error);}
  return res.json() as Promise<T>;
}
export const post=<T>(path:string,value:unknown)=>api<T>(path,{method:'POST',body:JSON.stringify(value)});
export const patch=<T>(path:string,value:unknown)=>api<T>(path,{method:'PATCH',body:JSON.stringify(value)});
export function q(params:Record<string,string|number>) {return '?'+new URLSearchParams(Object.entries(params).map(([k,v])=>[k,String(v)])).toString();}
export async function download(id:string,format:'docx'|'pdf') {
  const res=await fetch(`/api/sector-documents/${id}/export/${format}`,{headers:{Authorization:'Bearer '+token()}});
  if(!res.ok) throw new Error(await res.text());
  const blob=await res.blob();const url=URL.createObjectURL(blob);const a=document.createElement('a');a.href=url;a.download=`izvjesce-${id}.${format}`;a.click();setTimeout(()=>URL.revokeObjectURL(url),2000);
}
