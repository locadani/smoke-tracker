// The "which event?" question page posts once however often an answer is tapped. Reaching the
// real question needs a particular history, so the page is a stand-in with the same form, loaded
// with the app's own script.
import { expect, test } from "@playwright/test";

test("a_double_tap_on_an_answer_sends_one_post", async ({ page }) => {
  await page.route("**/question-stand-in", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: `<!doctype html><title>q</title>
<script src="/static/js/submit-once.js" type="module"></script>
<form method="post" action="/log">
  <input type="hidden" name="asked" value="1">
  <button type="submit" name="choice" value="new">New</button>
  <button type="submit" name="choice" value="join-1">Join</button>
</form>`,
    }),
  );
  const bodies = [];
  await page.route("**/log", async (route) => {
    bodies.push(route.request().postData());
    await new Promise((resolve) => setTimeout(resolve, 300));
    return route.fulfill({ status: 200, contentType: "text/html", body: "<p>done</p>" });
  });
  await page.goto("/question-stand-in");
  await page.evaluate(() => {
    document.addEventListener("submit", () => {
      setTimeout(() => document.querySelector("button[value=new]").form.requestSubmit(), 50);
    });
  });
  await page.click("button[value=join-1]", { noWaitAfter: true });
  await expect(page.locator("p", { hasText: "done" })).toBeVisible();
  expect(bodies).toHaveLength(1);
  expect(bodies[0]).toContain("choice=join-1");
});
