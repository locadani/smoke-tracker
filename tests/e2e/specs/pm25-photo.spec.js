// The photo controls of the Log screen, driven in a browser with a synthetic JPEG drawn in the
// page. The camera, the torch and the rotation of a real phone are not reachable from here.
import { expect, test } from "@playwright/test";

const LIMIT = 307200;

/** A noisy 4000x3000 JPEG that cannot fit the limit at full size. */
async function syntheticPhoto(page) {
  // base64 text: it crosses the page boundary far faster than an array of bytes.
  return page.evaluate(async () => {
    const canvas = document.createElement("canvas");
    canvas.width = 4000;
    canvas.height = 3000;
    const context = canvas.getContext("2d");
    context.fillStyle = "#223";
    context.fillRect(0, 0, 4000, 3000);
    for (let i = 0; i < 3000; i++) {
      context.fillStyle = `rgb(${Math.random() * 255} ${Math.random() * 255} ${Math.random() * 255})`;
      context.fillRect(Math.random() * 4000, Math.random() * 3000, 40, 40);
    }
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/jpeg", 0.95));
    const reader = new FileReader();
    return new Promise((resolve) => {
      reader.onload = () => resolve(reader.result.split(",")[1]);
      reader.readAsDataURL(blob);
    });
  });
}

async function choose(page, bytes) {
  await page.locator("input[name=photo]").setInputFiles({
    name: "display.jpg",
    mimeType: "image/jpeg",
    buffer: Buffer.from(bytes, "base64"),
  });
}

test.describe("photo on the log form", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // No torch in the test browser: the button opens the file input directly.
  });

  test("choosing_a_photo_previews_it_and_fills_nothing_without_a_reader", async ({ page }) => {
    await page.route("**/log/pm25/read", (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: '{"value":null}' }),
    );
    await choose(page, await syntheticPhoto(page));
    await expect(page.locator("[data-photo-preview]")).toBeVisible();
    await expect(page.locator("[data-photo-preview-image]")).toHaveAttribute("src", /^blob:/);
    await expect(page.locator("[data-photo-read-none]")).toBeVisible();
    await expect(page.locator("[data-photo-read-ok]")).toBeHidden();
    await expect(page.locator("[data-photo-attached]")).toBeVisible();
    await expect(page.locator("[data-photo-retake]")).toBeVisible();
    await expect(page.locator("[data-photo-remove]")).toBeVisible();
    await expect(page.locator("input[name=pm25]")).toHaveValue("");
    await expect(page.locator("input[name=photo_taken_at]")).toHaveValue(
      /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?Z$/,
    );
  });

  test("a_read_number_fills_an_empty_field_and_never_overwrites_typing", async ({ page }) => {
    await page.route("**/log/pm25/read", (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: '{"value":12.5}' }),
    );
    const field = page.locator("input[name=pm25]");
    await choose(page, await syntheticPhoto(page));
    await expect(field).toHaveValue("12.5");
    await expect(field).toHaveAttribute("data-from-photo", "");
    await expect(page.locator("[data-photo-read-ok]")).toBeVisible();

    await page.route("**/log/pm25/read", (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: '{"value":30}' }),
    );
    await page
      .locator("[data-photo-retake]")
      .click()
      .catch(() => {});
    await choose(page, await syntheticPhoto(page));
    await expect(field).toHaveValue("30");

    await field.fill("7");
    await expect(field).not.toHaveAttribute("data-from-photo", "");
    await choose(page, await syntheticPhoto(page));
    await expect(page.locator("[data-photo-preview-image]")).toHaveAttribute("src", /^blob:/);
    await expect(field).toHaveValue("7");
  });

  test("a_failing_reader_shows_the_none_message", async ({ page }) => {
    await page.route("**/log/pm25/read", (route) => route.fulfill({ status: 502, body: "" }));
    await choose(page, await syntheticPhoto(page));
    await expect(page.locator("[data-photo-read-none]")).toBeVisible();
  });

  test("the_uploaded_photo_is_at_most_the_limit", async ({ page }) => {
    await page.route("**/log/pm25/read", (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: '{"value":null}' }),
    );
    let body = null;
    await page.route("**/log", (route) => {
      if (route.request().method() === "POST") {
        body = route.request().postDataBuffer();
      }
      return route.fulfill({ status: 200, contentType: "text/html", body: "<p>ok</p>" });
    });
    await choose(page, await syntheticPhoto(page));
    await expect(page.locator("[data-photo-preview-image]")).toHaveAttribute("src", /^blob:/);
    const size = await page.evaluate(
      () => document.querySelector("input[name=photo]").files[0].size,
    );
    expect(size).toBeGreaterThan(0);
    expect(size).toBeLessThanOrEqual(LIMIT);
    expect(
      await page.evaluate(() => document.querySelector("input[name=photo]").files[0].type),
    ).toBe("image/jpeg");

    await page.fill("input[name=pm25]", "18");
    await page.click("button[value=log]");
    await expect.poll(() => body).not.toBeNull();
    expect(body.length).toBeLessThanOrEqual(LIMIT + 16 * 1024);
    expect(body.includes(Buffer.from([0xff, 0xd8, 0xff]))).toBe(true);
  });

  test("the_ladder_alone_returns_a_jpeg_within_the_limit", async ({ page }) => {
    const result = await page.evaluate(async (limit) => {
      const { shrinkToLimit } = await import("/static/js/pm25-photo.js");
      const canvas = document.createElement("canvas");
      canvas.width = 4000;
      canvas.height = 3000;
      const context = canvas.getContext("2d");
      for (let i = 0; i < 3000; i++) {
        context.fillStyle = `rgb(${Math.random() * 255} ${Math.random() * 255} ${Math.random() * 255})`;
        context.fillRect(Math.random() * 4000, Math.random() * 3000, 40, 40);
      }
      const source = await new Promise((resolve) => canvas.toBlob(resolve, "image/jpeg", 0.95));
      const shrunk = await shrinkToLimit(source, {
        maxBytes: limit,
        takenAt: new Date(),
        timeZone: "Europe/Rome",
      });
      return {
        quality: shrunk.quality,
        height: shrunk.height,
        size: shrunk.blob.size,
        type: shrunk.blob.type,
        fits: shrunk.fits,
        width: shrunk.width,
      };
    }, LIMIT);
    expect(result.fits).toBe(true);
    expect(result.size).toBeLessThanOrEqual(LIMIT);
    expect(result.type).toBe("image/jpeg");
    expect(result.width).toBeLessThanOrEqual(2400);
  });

  test("a_photo_that_cannot_fit_is_refused_with_the_page_message", async ({ page }) => {
    await page.route("**/log/pm25/read", (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: '{"value":null}' }),
    );
    await page.locator("[data-pm25-photo]").evaluate((button) => {
      button.dataset.maxBytes = "500";
    });
    await choose(page, await syntheticPhoto(page));
    await expect(page.locator("[data-photo-invalid]")).toBeVisible();
    await expect(page.locator("[data-photo-preview]")).toBeHidden();
    expect(
      await page.evaluate(() => document.querySelector("input[name=photo]").files.length),
    ).toBe(0);
  });

  test("remove_clears_the_photo", async ({ page }) => {
    await page.route("**/log/pm25/read", (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: '{"value":null}' }),
    );
    await choose(page, await syntheticPhoto(page));
    await expect(page.locator("[data-photo-remove]")).toBeVisible();
    await page.locator("[data-photo-remove]").click();
    await expect(page.locator("[data-photo-preview]")).toBeHidden();
    await expect(page.locator("[data-photo-attached]")).toBeHidden();
    await expect(page.locator("[data-photo-remove]")).toBeHidden();
    await expect(page.locator("[data-photo-retake]")).toBeHidden();
    await expect(page.locator("[data-photo-read-none]")).toBeHidden();
    expect(
      await page.evaluate(() => document.querySelector("input[name=photo]").files.length),
    ).toBe(0);
    await expect(page.locator("input[name=photo_taken_at]")).toHaveCount(0);
  });

  test("remove_clears_the_reference_fields_of_a_re_rendered_draft", async ({ page }) => {
    await page.evaluate(() => {
      const form = document.querySelector("form[action='/log']");
      for (const [name, value] of [
        ["photo_sha256", "a".repeat(64)],
        ["photo_bytes", "1234"],
        ["photo_taken_at", "2026-11-02T10:00:00+01:00"],
      ]) {
        const input = document.createElement("input");
        input.type = "hidden";
        input.name = name;
        input.value = value;
        form.prepend(input);
      }
      document.querySelector("[data-photo-attached]").hidden = false;
      document.querySelector("[data-photo-remove]").hidden = false;
    });
    await page.locator("[data-photo-remove]").click();
    for (const name of ["photo_sha256", "photo_bytes", "photo_taken_at"]) {
      await expect(page.locator(`input[name=${name}]`)).toHaveCount(0);
    }
    await expect(page.locator("[data-photo-attached]")).toBeHidden();
  });

  test("the_log_button_disables_on_submit", async ({ page }) => {
    // A page under a pending navigation answers no queries, so the page reports by itself, 50 ms
    // after the submit event: whether the button is disabled, and then a second submit.
    const seen = [];
    await page.exposeFunction("report", (what) => seen.push(what));
    await page.evaluate(() => {
      document.addEventListener("submit", () => {
        setTimeout(() => {
          report(`disabled:${document.querySelector("button[value=log]").disabled}`);
          document.querySelector("form[action='/log']").requestSubmit();
        }, 50);
      });
    });
    let posts = 0;
    await page.route("**/log", async (route) => {
      if (route.request().method() !== "POST") {
        return route.continue();
      }
      posts += 1;
      await new Promise((resolve) => setTimeout(resolve, 500));
      return route.fulfill({ status: 200, contentType: "text/html", body: "<p>ok</p>" });
    });
    await page.fill("input[name=pm25]", "18");
    await page.click("button[value=log]", { noWaitAfter: true });
    await expect(page.locator("p", { hasText: "ok" })).toBeVisible();
    expect(seen).toContain("disabled:true");
    expect(posts).toBe(1);
  });
});
