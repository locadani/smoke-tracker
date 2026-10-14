// Draws the charts of the Graphs screen (spec §9 screen 5) with the vendored Chart.js. The
// server writes each chart's configuration as JSON into its canvas's data-chart attribute;
// this file adds only what JSON cannot carry: the colours, read from the stylesheet's tokens
// (a dataset names its token) and read again when the system switches between light and dark,
// and labels for the time axis, written in the app's time zone (decision #95).
import "./series-switch.js";
import "./person-filter.js";

Chart.defaults.animation = false;
Chart.defaults.maintainAspectRatio = false;
Chart.defaults.plugins.legend.labels.filter = (item) => !item.hidden;

const dark = matchMedia("(prefers-color-scheme: dark)");

function token(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(`--${name}`).trim();
}

function paint(config) {
  Chart.defaults.color = token("muted");
  Chart.defaults.borderColor = token("line");
  for (const dataset of config.data.datasets) {
    if (dataset.token) {
      dataset.borderColor = token(dataset.token);
      dataset.backgroundColor = token(dataset.token);
    }
  }
}

function formatter(time, withClock) {
  const clock = withClock ? { hour: "2-digit", minute: "2-digit", hourCycle: "h23" } : {};
  return new Intl.DateTimeFormat(time.locale, {
    timeZone: time.zone,
    day: "2-digit",
    month: "2-digit",
    ...clock,
  });
}

function withTimeAxis(config, time) {
  const day = formatter(time, false);
  const moment = formatter(time, true);
  const x = config.options.scales.x;
  x.ticks = { ...x.ticks, callback: (value) => day.format(value) };
  x.afterBuildTicks = (scale) => {
    scale.ticks = time.ticks.map((value) => ({ value }));
  };
  config.options.plugins = {
    ...config.options.plugins,
    tooltip: { callbacks: { title: (items) => moment.format(items[0].parsed.x) } },
  };
}

const charts = [];
for (const canvas of document.querySelectorAll("canvas[data-chart]")) {
  try {
    const config = JSON.parse(canvas.dataset.chart);
    const time = config.time;
    delete config.time;
    if (time) {
      withTimeAxis(config, time);
    }
    paint(config);
    charts.push(new Chart(canvas, config));
  } catch (error) {
    console.error(error);
  }
}

dark.addEventListener("change", () => {
  for (const chart of charts) {
    paint(chart.config);
    chart.update("none");
  }
});
