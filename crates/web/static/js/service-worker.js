// Registers the service worker (spec §9.1), which keeps the static files and an offline page.
if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw.js");
}
