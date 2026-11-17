import {useEffect,useState} from 'react';
import {Download,Smartphone,X} from 'lucide-react';

type BeforeInstallPromptEvent=Event&{
 prompt:()=>Promise<void>;
 userChoice:Promise<{outcome:'accepted'|'dismissed';platform:string}>;
};

export default function PwaInstallButton(){
 const [installEvent,setInstallEvent]=useState<BeforeInstallPromptEvent|null>(null);
 const [showIosHelp,setShowIosHelp]=useState(false);
 const [installed,setInstalled]=useState(false);
 const isIos=/iphone|ipad|ipod/i.test(navigator.userAgent);
 useEffect(()=>{
  if('serviceWorker' in navigator){
   void navigator.serviceWorker.register('/sw.js',{scope:'/'}).catch(error=>console.warn('ReportFlow service worker registration failed',error));
  }
  const standalone=window.matchMedia('(display-mode: standalone)').matches||Boolean((navigator as Navigator&{standalone?:boolean}).standalone);
  setInstalled(standalone);
  const onPrompt=(event:Event)=>{event.preventDefault();setInstallEvent(event as BeforeInstallPromptEvent);};
  const onInstalled=()=>{setInstalled(true);setInstallEvent(null);setShowIosHelp(false);};
  window.addEventListener('beforeinstallprompt',onPrompt);
  window.addEventListener('appinstalled',onInstalled);
  return ()=>{window.removeEventListener('beforeinstallprompt',onPrompt);window.removeEventListener('appinstalled',onInstalled);};
 },[]);
 async function install(){
  if(installEvent){await installEvent.prompt();const choice=await installEvent.userChoice;if(choice.outcome==='accepted')setInstalled(true);setInstallEvent(null);return;}
  if(isIos)setShowIosHelp(value=>!value);
 }
 if(installed||(!installEvent&&!isIos))return null;
 return <div className="rf-pwa-install-wrap">
  <button type="button" className="outline rf-pwa-install" onClick={()=>void install()}><Download size={16}/>{installEvent?'Instaliraj aplikaciju':'Kako instalirati'}</button>
  {showIosHelp&&<div className="rf-pwa-help" role="dialog" aria-label="Instalacija ReportFlowa">
   <button type="button" className="rf-pwa-help-close" aria-label="Zatvori" onClick={()=>setShowIosHelp(false)}><X size={16}/></button>
   <strong><Smartphone size={16}/> Dodaj ReportFlow na početni zaslon</strong>
   <ol><li>U Safariju dodirni gumb <strong>Dijeli</strong>.</li><li>Odaberi <strong>Dodaj na početni zaslon</strong>.</li><li>Potvrdi s <strong>Dodaj</strong>.</li></ol>
  </div>}
 </div>;
}
