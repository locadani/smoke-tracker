// The switches above the intensity chart: Both, Intensity or PM2.5, and By person or Everyone.
// Each dataset names its group ("person", "everyone", "pm25"); a switch shows the groups it
// asks for and sets the chart's heading. Without JavaScript the switches stay hidden.
const visible = {
  both: ["lines", "pm25"],
  intensity: ["lines"],
  pm25: ["pm25"],
};

function apply(card) {
  const chart = Chart.getChart(card.querySelector("canvas"));
  if (!chart) {
    return;
  }
  const series = card.querySelector('input[name="series"]:checked');
  const lines = card.querySelector('input[name="lines"]:checked')?.value ?? "person";
  const shown = visible[series?.value ?? "both"];
  for (const dataset of chart.data.datasets) {
    const group = dataset.group === "pm25" ? "pm25" : "lines";
    const line = dataset.group === "pm25" || dataset.group === lines;
    dataset.hidden = !(shown.includes(group) && line);
  }
  if (series?.dataset.title) {
    card.querySelector("[data-series-title]").textContent = series.dataset.title;
    chart.canvas.setAttribute("aria-label", series.dataset.title);
  }
  chart.update("none");
}

for (const card of document.querySelectorAll("[data-series-chart]")) {
  for (const control of card.querySelectorAll("[data-series], [data-lines]")) {
    control.hidden = false;
  }
}

document.addEventListener("change", (event) => {
  const card = event.target.closest?.("[data-series-chart]");
  if (card) {
    apply(card);
  }
});
