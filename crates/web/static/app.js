// Progressive enhancement, loaded by every page. Links with a data-dialog attribute open that
// <dialog> instead of navigating (without JavaScript they open the full page), and tapping a
// dialog's backdrop closes it. The close button is a method="dialog" form and needs no script.
document.addEventListener("click", (event) => {
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
