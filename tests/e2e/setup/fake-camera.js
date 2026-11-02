// A stand-in for the phone camera in the camera specs: a plain green canvas stream that reports
// the capabilities a spec asks for, and records the zoom the page asks the camera for.

/**
 * Replaces the camera with a `width`x`height` canvas stream. `capabilities` is what the track
 * reports (`{ torch: true }` by default); with a `zoom` range the track also takes zoom settings
 * and keeps each one asked for in `window.zoomRequests`. Two extra `zoom` fields shape the fake:
 * `start`, the zoom it opens at, and `refuseAbove`, past which it refuses a setting.
 */
export async function fakeCamera(page, width, height, capabilities = { torch: true }) {
  await page.addInitScript(
    ([w, h, reported]) => {
      const canvas = document.createElement("canvas");
      canvas.width = w;
      canvas.height = h;
      const context = canvas.getContext("2d");
      const draw = () => {
        context.fillStyle = "rgb(0 200 0)";
        context.fillRect(0, 0, w, h);
        requestAnimationFrame(draw);
      };
      draw();
      // A canvas stream is not a camera, so autoplay rules would stop it.
      const play = HTMLMediaElement.prototype.play;
      HTMLMediaElement.prototype.play = function () {
        return play.call(this).catch(() => undefined);
      };
      window.zoomRequests = [];
      Object.defineProperty(MediaDevices.prototype, "getUserMedia", {
        configurable: true,
        value: async () => {
          const stream = canvas.captureStream(30);
          const track = stream.getVideoTracks()[0];
          track.getCapabilities = () => reported;
          if (reported.zoom) {
            let zoom = reported.zoom.start ?? reported.zoom.min;
            const settings = track.getSettings.bind(track);
            track.getSettings = () => ({ ...settings(), zoom });
            track.applyConstraints = async ({ advanced }) => {
              window.zoomRequests.push(advanced[0].zoom);
              if (advanced[0].zoom > (reported.zoom.refuseAbove ?? Infinity)) {
                throw new DOMException("zoom refused", "OverconstrainedError");
              }
              zoom = advanced[0].zoom;
            };
          }
          return stream;
        },
      });
    },
    [width, height, capabilities],
  );
}

/**
 * WebKit on Linux never reports the frame size of the first camera stream a browser opens (later
 * ones are fine; real iPhones are not affected), so each WebKit browser opens a throwaway one
 * first. Use as `test.beforeAll(warmUpWebKit)`.
 */
export async function warmUpWebKit({ browser, browserName }, info) {
  if (browserName !== "webkit") {
    return;
  }
  const context = await browser.newContext({ baseURL: info.project.use.baseURL });
  const page = await context.newPage();
  await fakeCamera(page, 640, 480);
  await page.goto("/login");
  await page.evaluate(async () => {
    const video = document.createElement("video");
    video.muted = true;
    video.srcObject = await navigator.mediaDevices.getUserMedia({ video: true });
    document.body.append(video);
    // Not awaited: on that first stream play() never settles either.
    video.play();
    await new Promise((resolve) => setTimeout(resolve, 1000));
  });
  await context.close();
}

/** Opens the in-page camera on the current page; its result lands in `window.shot`. */
export async function openCamera(page) {
  await page.evaluate(async () => {
    const { capture } = await import("/static/js/pm25-camera.js");
    window.shot = undefined;
    capture({
      timeZone: "Europe/Rome",
      hint: "Point the camera at the display",
      labels: { close: "Close", shutter: "Take photo", torch: "Torch" },
    }).then((result) => {
      window.shot = result;
    });
  });
}
