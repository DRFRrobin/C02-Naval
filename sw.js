/* C02 Naval : service worker.
   - La page est servie en priorité depuis le réseau (les mises à jour du jeu arrivent tout de suite)
     et, hors ligne, depuis le cache : le jeu s'ouvre et on peut jouer contre le bot.
   - Les bibliothèques et polices (CDN) sont gardées en cache après la première visite.
   - Rien d'autre n'est intercepté : le serveur PeerJS, les WebSockets et le TURN passent directement. */
const VERSION = 'c02-naval-v1.2.0';   // = APP_VERSION d'index.html (vérifié par les tests)
const SHELL = ['./', 'index.html', 'manifest.webmanifest', 'icons/icon-192.png', 'icons/icon-512.png', 'icons/apple-touch-icon.png'];
const CDN = ['https://unpkg.com/peerjs@1.5.4/dist/peerjs.min.js',
  'https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.2/css/all.min.css'];
const CDN_HOSTS = ['unpkg.com', 'cdnjs.cloudflare.com', 'fonts.googleapis.com', 'fonts.gstatic.com'];

self.addEventListener('install', e => {
  e.waitUntil((async () => {
    const c = await caches.open(VERSION);
    await Promise.all(SHELL.map(u => c.add(new Request(u, { cache: 'reload' })).catch(() => {})));
    await Promise.all(CDN.map(u => fetch(u, { mode: 'no-cors' }).then(r => c.put(u, r)).catch(() => {})));
    await self.skipWaiting();
  })());
});

self.addEventListener('activate', e => {
  e.waitUntil((async () => {
    for (const k of await caches.keys()) if (k !== VERSION) await caches.delete(k);
    await self.clients.claim();
  })());
});

self.addEventListener('fetch', e => {
  const req = e.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin === location.origin && url.pathname.endsWith('/version.json')) return;   // toujours lu directement sur le réseau
  if (url.origin === location.origin) {
    // même origine : réseau d'abord, cache en secours
    e.respondWith((async () => {
      const c = await caches.open(VERSION);
      try {
        const r = await fetch(req, { cache: 'no-cache' });   // on revalide : pas de vieille copie du cache HTTP
        if (r && r.ok) c.put(req.mode === 'navigate' ? 'index.html' : req, r.clone());
        return r;
      } catch (err) {
        return (await c.match(req, { ignoreSearch: true })) || (req.mode === 'navigate' ? await c.match('index.html') : null) || Response.error();
      }
    })());
  } else if (CDN_HOSTS.includes(url.hostname)) {
    // bibliothèques et polices : cache d'abord, mise à jour en arrière-plan
    e.respondWith((async () => {
      const c = await caches.open(VERSION);
      const hit = await c.match(req);
      const net = fetch(req).then(r => { if (r && (r.ok || r.type === 'opaque')) c.put(req, r.clone()); return r; }).catch(() => null);
      return hit || (await net) || Response.error();
    })());
  }
});
