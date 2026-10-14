// The service worker (spec §9.1), served at /sw.js so its scope is the whole app. It keeps the
// static assets and an offline page. Pages and form posts always go to the network: there is
// no offline logging. Assets are fetched first and the cached copy is the fallback, so a new
// deploy is never hidden behind an old cache.
const CACHE = "smoke-tracker-v2";
const PRECACHE = [
  "/offline",
  "/static/app.css",
  "/static/icons.svg",
  "/static/js/app.js",
  "/static/js/date-picks.js",
  "/static/js/dialog.js",
  "/static/js/install.js",
  "/static/js/service-worker.js",
  "/static/htmx-2.0.11.min.js",
  "/static/favicon.svg",
  "/static/icon-192.png",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin) {
    return;
  }
  if (request.mode === "navigate") {
    event.respondWith(fetch(request).catch(() => caches.match("/offline")));
  } else if (url.pathname.startsWith("/static/")) {
    event.respondWith(
      fetch(request)
        .then((response) => {
          if (response.ok) {
            const copy = response.clone();
            event.waitUntil(caches.open(CACHE).then((cache) => cache.put(request, copy)));
          }
          return response;
        })
        .catch(() => caches.match(request)),
    );
  }
});
