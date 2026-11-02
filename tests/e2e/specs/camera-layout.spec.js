// The in-page camera screen at phone size: the guide box, the clock and the buttons must be drawn
// above the live picture and stay on screen. Runs in WebKit (iPhone) and Chromium (Android) with a
// fake camera; the notch, the torch and a real sensor are not reachable from here.
import { expect, test } from "@playwright/test";

/** Replaces the camera with a 4K canvas stream that reports a torch. */
async function fakeCamera(page, width, height) {
  await page.addInitScript(
    ([w, h]) => {
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
      Object.defineProperty(MediaDevices.prototype, "getUserMedia", {
        configurable: true,
        value: async () => {
          const stream = canvas.captureStream(30);
          stream.getVideoTracks()[0].getCapabilities = () => ({ torch: true });
          return stream;
        },
      });
    },
    [width, height],
  );
}

/** The colour of the screen at a point, as [r, g, b]. */
async function colourAt(page, x, y) {
  const png = await page.screenshot({ clip: { x, y, width: 1, height: 1 }, scale: "css" });
  return page.evaluate(async (base64) => {
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
    const canvas = new OffscreenCanvas(1, 1);
    const context = canvas.getContext("2d");
    context.drawImage(bitmap, 0, 0);
    return [...context.getImageData(0, 0, 1, 1).data.slice(0, 3)];
  }, png.toString("base64"));
}

const isGreen = ([r, g, b]) => g > 150 && r < 80 && b < 80;

// WebKit on Linux never reports the frame size of the first camera stream a browser opens (later
// ones are fine; real iPhones are not affected), so each WebKit browser opens a throwaway one first.
test.beforeAll(async ({ browser, browserName }, info) => {
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
});

async function box(page, selector) {
  return page.locator(selector).boundingBox();
}

for (const [width, height] of [
  [3840, 2160],
  [2160, 3840],
]) {
  test(`camera_controls_show_above_a_${width}x${height}_picture`, async ({ page }) => {
    await fakeCamera(page, width, height);
    await page.goto("/login");
    await page.evaluate(async () => {
      const { capture } = await import("/static/js/pm25-camera.js");
      capture({
        timeZone: "Europe/Rome",
        hint: "Point the camera at the display",
        labels: { close: "Close", shutter: "Take photo", torch: "Torch" },
      });
    });
    await expect(page.locator(".camera-shutter")).toBeVisible();
    await expect
      .poll(async () => (await page.locator(".camera-video").evaluate((v) => v.videoWidth)) > 0)
      .toBe(true);

    const viewport = page.viewportSize();
    const video = await box(page, ".camera-video");
    const bar = await box(page, ".camera-bar");
    for (const part of [video, bar]) {
      expect(part.x).toBeGreaterThanOrEqual(0);
      expect(part.y).toBeGreaterThanOrEqual(0);
      expect(part.x + part.width).toBeLessThanOrEqual(viewport.width + 0.5);
      expect(part.y + part.height).toBeLessThanOrEqual(viewport.height + 0.5);
    }
    const sideBySide = viewport.width > viewport.height;
    if (sideBySide) {
      expect(video.x + video.width).toBeLessThanOrEqual(bar.x + 0.5);
    } else {
      expect(video.y + video.height).toBeLessThanOrEqual(bar.y + 0.5);
    }

    // The picture is plain green: any of these pixels being green means the video covers it.
    const shutter = await box(page, ".camera-shutter");
    const clock = await box(page, ".camera-clock");
    const guide = await box(page, ".camera-guide");
    const points = {
      shutter: [shutter.x + shutter.width / 2, shutter.y + shutter.height / 2],
      clock: [clock.x + 2, clock.y + clock.height / 2],
      guide: [guide.x + 1, guide.y + guide.height / 2],
    };
    for (const [name, [x, y]] of Object.entries(points)) {
      const colour = await colourAt(page, Math.round(x), Math.round(y));
      expect(isGreen(colour), `${name} at ${x},${y} is ${colour}`).toBe(false);
    }
  });
}
