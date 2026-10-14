// A link with data-dialog="<id>" opens that <dialog> in place of the page it leads to, and a tap
// on the backdrop closes it. The close button is a method="dialog" form and needs no script.
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
