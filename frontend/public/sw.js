const CACHE_NAME='reportflow-shell-v1';
const APP_SHELL=['/','/manifest.webmanifest','/icons/reportflow.svg'];

self.addEventListener('install',event=>{
 event.waitUntil(caches.open(CACHE_NAME).then(cache=>cache.addAll(APP_SHELL)).then(()=>self.skipWaiting()));
});

self.addEventListener('activate',event=>{
 event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key.startsWith('reportflow-shell-')&&key!==CACHE_NAME).map(key=>caches.delete(key)))).then(()=>self.clients.claim()));
});

self.addEventListener('fetch',event=>{
 const request=event.request;
 const url=new URL(request.url);
 if(request.method!=='GET'||url.origin!==self.location.origin||url.pathname.startsWith('/api/'))return;

 if(request.mode==='navigate'){
  event.respondWith(fetch(request).then(response=>{
   if(response.ok){const copy=response.clone();void caches.open(CACHE_NAME).then(cache=>cache.put('/',copy));}
   return response;
  }).catch(async()=>await caches.match('/')||await caches.match('/index.html')));
  return;
 }

 event.respondWith(caches.match(request).then(cached=>{
  if(cached)return cached;
  return fetch(request).then(response=>{
   if(response.ok&&response.type==='basic'){const copy=response.clone();void caches.open(CACHE_NAME).then(cache=>cache.put(request,copy));}
   return response;
  });
 }));
});
