// A <details data-menu> closes when the person taps outside it or presses Escape, and Escape
// gives the focus back to its summary. Without JavaScript it opens and closes from its summary.
const openMenus = () => document.querySelectorAll("details[data-menu][open]");

document.addEventListener("click", (event) => {
  for (const menu of openMenus()) {
    if (!menu.contains(event.target)) {
      menu.open = false;
    }
  }
});

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") {
    return;
  }
  for (const menu of openMenus()) {
    menu.open = false;
    menu.querySelector("summary")?.focus();
  }
});
