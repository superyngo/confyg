// App shell offline. Two rules, no cleverness:
//   - the shell (document, styles, module graph entry) is precached and served cache-first
//   - everything else is network-first with a cache fallback, so a stale wasm bundle can
//     never outlive a deploy while still working on a plane
const VERSION = "confyg-v1";
const SHELL = ["/", "/index.html", "/manifest.webmanifest"];

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(VERSION).then((c) => c.addAll(SHELL)));
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) => Promise.all(keys.filter((k) => k !== VERSION).map((k) => caches.delete(k)))),
  );
  self.clients.claim();
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") return;
  // A navigation always gets the shell, so a file-handler launch works offline.
  if (request.mode === "navigate") {
    event.respondWith(caches.match("/index.html").then((hit) => hit || fetch(request)));
    return;
  }
  event.respondWith(
    fetch(request)
      .then((response) => {
        const copy = response.clone();
        void caches.open(VERSION).then((c) => c.put(request, copy));
        return response;
      })
      .catch(() => caches.match(request)),
  );
});
