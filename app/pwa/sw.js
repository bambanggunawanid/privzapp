// PrivZapp service worker: makes the app fully offline-capable.
//
// Strategy: cache-first for same-origin GETs. dx fingerprints every asset
// (hashed filenames), so a cache hit can never be stale; new deploys ship new
// URLs. Navigations try the network first (to pick up new releases) and fall
// back to the cached shell when offline.
//
// This file must be served from the origin root so its scope covers "/".
// scripts/build-web.sh copies it (and the manifest/icons) into the bundle.

const CACHE = 'privzapp-v1.0.0';

self.addEventListener('install', (event) => {
  self.skipWaiting();
  event.waitUntil(caches.open(CACHE).then((cache) => cache.add('/')));
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || url.origin !== self.location.origin) {
    return; // never touch cross-origin traffic; the app makes none anyway
  }

  if (event.request.mode === 'navigate') {
    event.respondWith(
      fetch(event.request)
        .then((res) => {
          const copy = res.clone();
          caches.open(CACHE).then((cache) => cache.put('/', copy));
          return res;
        })
        .catch(() => caches.match('/'))
    );
    return;
  }

  event.respondWith(
    caches.match(event.request).then(
      (hit) =>
        hit ||
        fetch(event.request).then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(CACHE).then((cache) => cache.put(event.request, copy));
          }
          return res;
        })
    )
  );
});
