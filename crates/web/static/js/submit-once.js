// A form posts once: after the first submit its submit buttons are disabled, so a second tap
// sends nothing. The buttons are disabled after the event, when the browser has already read the
// pressed button's name and value.
for (const form of document.querySelectorAll("form")) {
  let sent = false;
  form.addEventListener("submit", (event) => {
    if (sent) {
      event.preventDefault();
      return;
    }
    sent = true;
    setTimeout(() => {
      for (const button of form.querySelectorAll("button[type=submit]")) {
        button.disabled = true;
      }
    }, 0);
  });
  addEventListener("pageshow", (event) => {
    if (event.persisted) {
      sent = false;
      for (const button of form.querySelectorAll("button[type=submit]")) {
        button.disabled = false;
      }
    }
  });
}
