// The in-page camera screen at phone size: the guide box, the clock and the buttons must be drawn
// above the live picture and stay on screen. Runs in WebKit (iPhone) and Chromium (Android) with a
// fake camera; the notch, the torch and a real sensor are not reachable from here.
import { expect, test } from "@playwright/test";
import { fakeCamera, openCamera, warmUpWebKit } from "../setup/fake-camera.js";

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

test.beforeAll(warmUpWebKit);

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
    await openCamera(page);
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
