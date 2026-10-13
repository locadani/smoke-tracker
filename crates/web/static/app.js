// Progressive enhancement, loaded by every page.
//
// - Links with a data-dialog attribute open that <dialog> instead of navigating (without
//   JavaScript they open the full page), and tapping a dialog's backdrop closes it. The close
//   button is a method="dialog" form and needs no script.
// - In the expiry fields, typing a date picks "at the end of this day", and picking any other
//   option clears the date.
// - The service worker is registered (spec §9.1).
// - Where the browser offers its own install prompt (Chromium, Android), every data-install
//   element opens it; a hidden install button is shown. Elsewhere the Install link leads to the
//   page with the steps (spec §8.3).
let installPrompt = null;

document.addEventListener("click", (event) => {
  const installer = event.target.closest("[data-install]");
  if (installer && installPrompt) {
    event.preventDefault();
    installPrompt.prompt();
    installPrompt = null;
    return;
  }
  const opener = event.target.closest("[data-dialog]");
  if (opener) {
    const dialog = document.getElementById(opener.dataset.dialog);
    if (dialog instanceof HTMLDialogElement) {
      event.preventDefault();
      dialog.showModal();
    }
    return;
  }
  if (event.target instanceof HTMLDialogElement) {
    event.target.close();
  }
});

document.addEventListener("input", (event) => {
  const field = event.target;
  if (field instanceof HTMLInputElement && field.type === "date" && field.value) {
    const radio = field.closest(".expiry")?.querySelector('input[type="radio"][value="date"]');
    if (radio) radio.checked = true;
  }
});

document.addEventListener("change", (event) => {
  const radio = event.target;
  if (radio instanceof HTMLInputElement && radio.type === "radio" && radio.value !== "date") {
    const fieldset = radio.closest(".expiry");
    const date = fieldset?.querySelector('input[type="date"]');
    if (date) date.value = "";
  }
});

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

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw.js");
}
