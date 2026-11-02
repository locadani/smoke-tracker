// Pinch-to-zoom on the in-page camera, with a fake 4K camera and two synthetic touch pointers.
// Runs in WebKit (iPhone) and Chromium (Android); a real lens and real fingers are not reachable
// from here.
import { expect, test } from "@playwright/test";
import { fakeCamera, openCamera, warmUpWebKit } from "../setup/fake-camera.js";

const WIDTH = 3840;
const HEIGHT = 2160;

test.beforeAll(warmUpWebKit);

/** Two fingers moving apart (or together) from `from` to `to` pixels on the viewfinder. */
async function pinch(page, from, to) {
  await page.locator(".camera").evaluate(
    (overlay, [start, end]) => {
      const box = overlay.getBoundingClientRect();
      const x = box.width / 2;
      const y = box.height / 3;
      const send = (type, pointerId, gap) =>
        overlay.dispatchEvent(
          new PointerEvent(type, {
            pointerId,
            pointerType: "touch",
            isPrimary: pointerId === 1,
            clientX: x + (pointerId === 1 ? -gap : gap) / 2,
            clientY: y,
            bubbles: true,
          }),
        );
      send("pointerdown", 1, start);
      send("pointerdown", 2, start);
      for (let step = 1; step <= 10; step++) {
        const gap = start + ((end - start) * step) / 10;
        send("pointermove", 1, gap);
        send("pointermove", 2, gap);
      }
      send("pointerup", 1, end);
      send("pointerup", 2, end);
    },
    [from, to],
  );
}

async function openReady(page) {
  await page.goto("/login");
  await openCamera(page);
  await expect(page.locator(".camera-shutter")).toBeVisible();
  await expect
    .poll(async () => (await page.locator(".camera-video").evaluate((v) => v.videoWidth)) > 0)
    .toBe(true);
}

async function shoot(page) {
  await page.locator(".camera-shutter").click();
  await expect.poll(() => page.evaluate(() => window.shot !== undefined)).toBe(true);
  return page.evaluate(async () => {
    const bitmap = await createImageBitmap(window.shot.blob);
    return { width: bitmap.width, height: bitmap.height };
  });
}

test("the_zoom_level_is_hidden_until_the_first_pinch", async ({ page }) => {
  await fakeCamera(page, WIDTH, HEIGHT);
  await openReady(page);
  await expect(page.locator(".camera-zoom")).toBeHidden();
  await expect(page.locator(".camera")).toHaveCSS("touch-action", "none");
});

test("a_pinch_zooms_a_camera_that_can_zoom_and_keeps_the_full_frame", async ({ page }) => {
  await fakeCamera(page, WIDTH, HEIGHT, { torch: true, zoom: { min: 1, max: 5, step: 0.1 } });
  await openReady(page);

  await pinch(page, 100, 200);

  await expect(page.locator(".camera-zoom")).toHaveText("2.0×");
  await expect.poll(() => page.evaluate(() => window.zoomRequests.at(-1))).toBeCloseTo(2, 1);
  await expect(page.locator(".camera-video")).toHaveCSS("transform", "none");
  expect(await shoot(page)).toEqual({ width: WIDTH, height: HEIGHT });
});

test("a_pinch_stops_at_the_camera_zoom_limits", async ({ page }) => {
  await fakeCamera(page, WIDTH, HEIGHT, { torch: true, zoom: { min: 1, max: 3, step: 0.1 } });
  await openReady(page);

  await pinch(page, 50, 300);
  await expect(page.locator(".camera-zoom")).toHaveText("3.0×");

  await pinch(page, 300, 20);
  await expect(page.locator(".camera-zoom")).toBeHidden();
  await expect.poll(() => page.evaluate(() => window.zoomRequests.at(-1))).toBe(1);
});

test("a_wider_lens_is_not_offered_and_the_level_counts_from_the_starting_view", async ({
  page,
}) => {
  await fakeCamera(page, WIDTH, HEIGHT, {
    torch: true,
    zoom: { min: 0.5, max: 10, step: 0.1, start: 1 },
  });
  await openReady(page);

  await pinch(page, 200, 100);
  await expect(page.locator(".camera-zoom")).toBeHidden();
  expect(await page.evaluate(() => window.zoomRequests)).toEqual([]);

  await pinch(page, 100, 250);
  await expect(page.locator(".camera-zoom")).toHaveText("2.5×");
  // Every setting sits on the camera's 0.1 steps.
  const requests = await page.evaluate(() => window.zoomRequests);
  for (const zoom of requests) {
    expect(Math.abs(zoom * 10 - Math.round(zoom * 10))).toBeLessThan(1e-9);
  }
});

test("a_camera_that_refuses_a_zoom_falls_back_to_the_screen_zoom", async ({ page }) => {
  await fakeCamera(page, WIDTH, HEIGHT, {
    torch: true,
    zoom: { min: 1, max: 5, step: 0.1, refuseAbove: 1.5 },
  });
  await openReady(page);

  await pinch(page, 100, 200);
  // The lens goes back to where it started and the level no longer counts it.
  await expect.poll(() => page.evaluate(() => window.zoomRequests.at(-1))).toBe(1);
  await expect(page.locator(".camera-zoom")).toBeHidden();

  await pinch(page, 100, 200);
  await expect(page.locator(".camera-zoom")).toHaveText("2.0×");
  await expect(page.locator(".camera-video")).toHaveCSS("transform", "matrix(2, 0, 0, 2, 0, 0)");
});

test("without_camera_zoom_a_pinch_scales_the_picture_and_crops_the_photo", async ({ page }) => {
  await fakeCamera(page, WIDTH, HEIGHT);
  await openReady(page);

  await pinch(page, 100, 200);

  await expect(page.locator(".camera-zoom")).toHaveText("2.0×");
  await expect(page.locator(".camera-video")).toHaveCSS("transform", "matrix(2, 0, 0, 2, 0, 0)");
  const view = await page.locator(".camera-view").boundingBox();
  const expected = await page.evaluate(
    async ([frame, box]) => {
      const { visibleCrop } = await import("/static/js/pm25-camera-zoom.js");
      return visibleCrop(frame, box, 2);
    },
    [
      { width: WIDTH, height: HEIGHT },
      { width: view.width, height: view.height },
    ],
  );
  const photo = await shoot(page);
  expect(photo.width).toBeLessThan(WIDTH);
  expect(Math.abs(photo.width - expected.width)).toBeLessThanOrEqual(1);
  expect(Math.abs(photo.height - expected.height)).toBeLessThanOrEqual(1);
});

test("the_visible_crop_follows_the_letterboxed_frame", async ({ page }) => {
  await page.goto("/login");
  const crops = await page.evaluate(async () => {
    const { visibleCrop } = await import("/static/js/pm25-camera-zoom.js");
    const frame = { width: 4000, height: 2000 };
    return [
      // A wide frame in a tall box: at 2x the sides are cut, the height still fits.
      visibleCrop(frame, { width: 400, height: 800 }, 2),
      // At 1x nothing is cut.
      visibleCrop(frame, { width: 400, height: 800 }, 1),
      // A box of the same shape: both sides are halved.
      visibleCrop(frame, { width: 400, height: 200 }, 2),
    ];
  });
  expect(crops).toEqual([
    { x: 1000, y: 0, width: 2000, height: 2000 },
    { x: 0, y: 0, width: 4000, height: 2000 },
    { x: 1000, y: 500, width: 2000, height: 1000 },
  ]);
});
