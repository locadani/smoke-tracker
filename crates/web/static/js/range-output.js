// A range input with data-output="<id>" writes its value into that <output> as it moves: its
// data-zero text for 0 ("any"), else "5+". Without JavaScript the output shows the value the
// page was loaded with.
document.addEventListener("input", (event) => {
  const range = event.target;
  if (!(range instanceof HTMLInputElement) || !range.dataset.output) {
    return;
  }
  const output = document.getElementById(range.dataset.output);
  if (output) {
    output.value = range.value === "0" ? (range.dataset.zero ?? "0") : `${range.value}+`;
    range.setAttribute("aria-valuetext", output.value);
  }
});
