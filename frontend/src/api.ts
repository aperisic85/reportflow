export type User = {id:string;username:string;name:string;role:'admin'|'org_admin'|'director'|'manager'|'employee';department_id:string|null;sector_id:string|null};
export type Dept = {id:string;code:string;name:string;sort_order:number;sector_id:string};
export type Sector = {id:string;code:string;name:string};
export type Activity = {id:string;department_id:string;parent_id:string|null;title:string;description:string;type:string;status:string;sort_order:number;created_by:string|null;created_by_name?:string|null;created_at?:string;updated_at?:string};
export type Update = {id:string;activity_id:string;content:string;progress:string;include_in_document:boolean;created_at?:string;updated_at?:string;created_by_name?:string|null;updated_by_name?:string|null;creation_time_estimated?:boolean};
export type SectorItem = {id:string;title:string;description:string;content:string;parent_id:string|null;type?:string;progress?:string};
export type SectorSection = {department_id:string;code:string;name:string;source_version?:number|null;source_submitted_at?:string|null;source_submitted_by?:string|null;items:SectorItem[]};
export type SectorSnapshot = {year:number;month:number;kind:string;title:string;sections:SectorSection[]};
export type SectorDoc = {id:string;title:string;status:string;revision:number;created_at:string;created_by_name?:string|null;snapshot:SectorSnapshot;introduction:string;conclusion:string};
export type PlanReference = {activity_id:string;title:string;planned:string};
const token = () => sessionStorage.getItem('rf_token');
export async function api<T>(path:string, options:RequestInit={}):Promise<T> {
  const res=await fetch('/api'+path,{...options,headers:{...(options.body?{'Content-Type':'application/json'}:{}),...(token()?{Authorization:'Bearer '+token()}:{}),...options.headers}});
  if (!res.ok) {let error=res.statusText;try {const data=await res.json();error=data.error??error;}catch { /* non-JSON error */ }throw new Error(error);}
  return res.json() as Promise<T>;
}
export const post=<T>(path:string,value:unknown)=>api<T>(path,{method:'POST',body:JSON.stringify(value)});
export const patch=<T>(path:string,value:unknown)=>api<T>(path,{method:'PATCH',body:JSON.stringify(value)});
export function q(params:Record<string,string|number>) {return '?'+new URLSearchParams(Object.entries(params).map(([k,v])=>[k,String(v)])).toString();}
export async function download(id:string,format:'docx'|'pdf'|'modern-pdf') {
  const res=await fetch(`/api/sector-documents/${id}/export/${format}`,{headers:{Authorization:'Bearer '+token()}});
  if(!res.ok) throw new Error(await res.text());
  const blob=await res.blob();const url=URL.createObjectURL(blob);const a=document.createElement('a');a.href=url;a.download=`izvjesce-${id}${format==='modern-pdf'?'-modern':''}.${format==='docx'?'docx':'pdf'}`;a.click();setTimeout(()=>URL.revokeObjectURL(url),2000);
}
