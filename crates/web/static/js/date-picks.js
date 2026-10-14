// A fieldset with data-date-picks offers quick picks (radio buttons) and date fields. Typing a
// date checks the radio whose value is "date", or clears the picks when there is none; choosing
// a pick clears the dates, so the form never sends both. Without JavaScript the server decides.
document.addEventListener("input", (event) => {
  const field = event.target;
  const picks = field.closest?.("[data-date-picks]");
  if (!picks || !(field instanceof HTMLInputElement) || field.type !== "date" || !field.value) {
    return;
  }
  const dated = picks.querySelector('input[type="radio"][value="date"]');
  for (const radio of picks.querySelectorAll('input[type="radio"]')) {
    radio.checked = radio === dated;
  }
});

document.addEventListener("change", (event) => {
  const radio = event.target;
  const picks = radio.closest?.("[data-date-picks]");
  if (
    picks &&
    radio instanceof HTMLInputElement &&
    radio.type === "radio" &&
    radio.value !== "date"
  ) {
    for (const date of picks.querySelectorAll('input[type="date"]')) {
      date.value = "";
    }
  }
});
