// Draws the charts of the Graphs screen (spec §9 screen 5) with the vendored Chart.js. The
// server writes each chart's configuration as JSON into its canvas's data-chart attribute; this
// file adds only what JSON cannot carry: labels for the time axis, written in the app's time
// zone (decision #95) rather than the browser's.
Chart.defaults.animation = false;
Chart.defaults.maintainAspectRatio = false;

function formatter(time, withClock) {
  const clock = withClock ? { hour: "2-digit", minute: "2-digit", hourCycle: "h23" } : {};
  return new Intl.DateTimeFormat(time.locale, {
    timeZone: time.zone,
    day: "2-digit",
    month: "2-digit",
    ...clock,
  });
}

for (const canvas of document.querySelectorAll("canvas[data-chart]")) {
  const config = JSON.parse(canvas.dataset.chart);
  const time = config.time;
  delete config.time;
  if (time) {
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
  new Chart(canvas, config);
}
