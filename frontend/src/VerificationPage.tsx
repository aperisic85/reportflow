import {useEffect,useState} from 'react';

type Verification={
 valid:boolean;document_number:string;sector:string;year:number;month:number;
 kind:string;revision:number;approved_at:string;approved_by:string;
 pdf_sha256:string;pdf_size:number;approval_method:string;notice:string;
};

// Web Crypto is unavailable on ordinary HTTP origins (for example, an internal
// http://192.168.x.x address). This small fallback hashes the file locally,
// without uploading it, so PDF verification also works in those environments.
function sha256Fallback(bytes:Uint8Array):string{
 const k=[
  0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
  0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
  0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
  0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
  0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
  0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
  0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
  0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
 ];
 const h=[0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
 const paddedLength=Math.ceil((bytes.length+9)/64)*64;
 const data=new Uint8Array(paddedLength);
 data.set(bytes);data[bytes.length]=0x80;
 const view=new DataView(data.buffer);
 const bitLength=bytes.length*8;
 view.setUint32(paddedLength-8,Math.floor(bitLength/0x100000000));
 view.setUint32(paddedLength-4,bitLength>>>0);
 const w=new Uint32Array(64);
 const rotr=(x:number,n:number)=>(x>>>n)|(x<<(32-n));
 for(let offset=0;offset<paddedLength;offset+=64){
  for(let i=0;i<16;i++)w[i]=view.getUint32(offset+i*4);
  for(let i=16;i<64;i++){
   const s0=rotr(w[i-15],7)^rotr(w[i-15],18)^(w[i-15]>>>3);
   const s1=rotr(w[i-2],17)^rotr(w[i-2],19)^(w[i-2]>>>10);
   w[i]=(w[i-16]+s0+w[i-7]+s1)>>>0;
  }
  let [a,b,c,d,e,f,g,hh]=h;
  for(let i=0;i<64;i++){
   const s1=rotr(e,6)^rotr(e,11)^rotr(e,25);
   const ch=(e&f)^(~e&g);
   const t1=(hh+s1+ch+k[i]+w[i])>>>0;
   const s0=rotr(a,2)^rotr(a,13)^rotr(a,22);
   const maj=(a&b)^(a&c)^(b&c);
   const t2=(s0+maj)>>>0;
   hh=g;g=f;f=e;e=(d+t1)>>>0;d=c;c=b;b=a;a=(t1+t2)>>>0;
  }
  h[0]=(h[0]+a)>>>0;h[1]=(h[1]+b)>>>0;h[2]=(h[2]+c)>>>0;h[3]=(h[3]+d)>>>0;
  h[4]=(h[4]+e)>>>0;h[5]=(h[5]+f)>>>0;h[6]=(h[6]+g)>>>0;h[7]=(h[7]+hh)>>>0;
 }
 return h.map(x=>x.toString(16).padStart(8,'0')).join('');
}
async function sha256Hex(bytes:ArrayBuffer):Promise<string>{
 if(typeof globalThis.crypto!=='undefined'&&globalThis.crypto.subtle){
  const digest=await globalThis.crypto.subtle.digest('SHA-256',bytes);
  return Array.from(new Uint8Array(digest)).map(x=>x.toString(16).padStart(2,'0')).join('');
 }
 return sha256Fallback(new Uint8Array(bytes));
}

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
   const hex=await sha256Hex(bytes);
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
    {checking&&<p role="status">Provjeravam SHA-256…</p>}
    {comparison&&<p className={comparison.startsWith('PODUDARA')?'verify-ok':'verify-bad'} role="status">{comparison}</p>}
   </section>
   <p className="verify-disclaimer">{result.notice} QR kod potvrđuje pristup zapisu u evidenciji, ne predstavlja kriptografski PAdES potpis.</p>
  </>}
 </main></div>;
}
