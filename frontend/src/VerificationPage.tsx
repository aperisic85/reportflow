import {useEffect,useState} from 'react';

type Verification={
 valid:boolean;document_number:string;sector:string;year:number;month:number;
 kind:string;revision:number;approved_at:string;approved_by:string;
 pdf_sha256:string;pdf_size:number;approval_method:string;notice:string;
};
export default function VerificationPage({token}:{token:string}){
 const [result,setResult]=useState<Verification|null>(null);
 const [error,setError]=useState('');
 const [comparison,setComparison]=useState('');
 const [checking,setChecking]=useState(false);
 useEffect(()=>{
  let alive=true;
  fetch('/api/verify/'+encodeURIComponent(token),{cache:'no-store'})
   .then(async response=>{if(!response.ok)throw Error('Dokument nije pronađen ili nije odobren.');return response.json() as Promise<Verification>;})
   .then(data=>{if(alive)setResult(data);})
   .catch(err=>{if(alive)setError(err instanceof Error?err.message:'Provjera nije uspjela.');});
  return()=>{alive=false;};
 },[token]);
 const compare=async(file:File)=>{
  setChecking(true);setComparison('');
  try{
   if(!result)throw Error('Podaci za provjeru nisu dostupni.');
   if(file.size!==result.pdf_size){setComparison('NE PODUDARA SE – veličina datoteke razlikuje se od arhiviranog PDF-a.');return;}
   const bytes=await file.arrayBuffer();
   const digest=await crypto.subtle.digest('SHA-256',bytes);
   const hex=Array.from(new Uint8Array(digest)).map(x=>x.toString(16).padStart(2,'0')).join('');
   setComparison(hex.toLowerCase()===result.pdf_sha256.toLowerCase()
    ?'PODUDARA SE – PDF je bajt-po-bajt jednak arhiviranom odobrenom dokumentu.'
    :'NE PODUDARA SE – SHA-256 razlikuje se od arhiviranog odobrenog dokumenta.');
  }catch(e){setComparison(e instanceof Error?e.message:'Provjera datoteke nije uspjela.');}
  finally{setChecking(false);}
 };
 return <div className="verify-page"><main className="verify-card">
  <header><strong className="verify-brand">ReportFlow</strong><h1>Provjera dokumenta</h1><p>Interna elektronička evidencija odobrenih izvješća</p></header>
  {error&&<div className="alert danger" role="alert">{error}</div>}
  {!error&&!result&&<p>Provjeravam identifikator dokumenta…</p>}
  {result&&<><div className="verify-success">✓ Dokument je evidentiran i interno odobren</div>
   <dl className="verify-details">
    <div><dt>Identifikator</dt><dd>{result.document_number}</dd></div>
    <div><dt>Sektor</dt><dd>{result.sector}</dd></div>
    <div><dt>Razdoblje</dt><dd>{String(result.month).padStart(2,'0')}/{result.year}</dd></div>
    <div><dt>Dokument</dt><dd>{result.kind==='plan'?'Plan':'Izvješće'} · revizija v{result.revision}</dd></div>
    <div><dt>Odobrio/la</dt><dd>{result.approved_by}</dd></div>
    <div><dt>Datum i vrijeme</dt><dd>{new Date(result.approved_at).toLocaleString('hr-HR',{dateStyle:'long',timeStyle:'medium'})}</dd></div>
    <div><dt>SHA-256 PDF-a</dt><dd className="verify-hash">{result.pdf_sha256}</dd></div>
   </dl>
   <section className="verify-file"><h2>Usporedi preuzeti PDF</h2>
    <p>Odaberi PDF za provjeru. Datoteka se obrađuje u tvom pregledniku i ne šalje na poslužitelj.</p>
    <input type="file" accept="application/pdf,.pdf" disabled={checking} onChange={e=>{const file=e.target.files?.[0];if(file)void compare(file);}}/>
    {comparison&&<p className={comparison.startsWith('PODUDARA')?'verify-ok':'verify-bad'} role="status">{comparison}</p>}
   </section>
   <p className="verify-disclaimer">{result.notice} QR kod potvrđuje pristup zapisu u evidenciji, ne predstavlja kriptografski PAdES potpis.</p>
  </>}
 </main></div>;
}
