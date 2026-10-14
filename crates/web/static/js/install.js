// Where the browser has its own install prompt (Chromium, Android), every data-install element
// opens it and hidden install buttons appear. Elsewhere data-install links lead to /install, the
// page with the steps (spec §8.3).
let installPrompt = null;

window.addEventListener("beforeinstallprompt", (event) => {
  event.preventDefault();
  installPrompt = event;
  for (const button of document.querySelectorAll("button[data-install]")) {
    button.hidden = false;
  }
});

window.addEventListener("appinstalled", () => {
  installPrompt = null;
  for (const element of document.querySelectorAll("[data-install]")) {
    element.hidden = true;
  }
});

document.addEventListener("click", (event) => {
  const installer = event.target.closest("[data-install]");
  if (installer && installPrompt) {
    event.preventDefault();
    installPrompt.prompt();
    installPrompt = null;
  }
});
