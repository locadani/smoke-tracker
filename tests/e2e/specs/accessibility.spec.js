// axe-core on the main pages, at phone size, in the light and the dark theme (one project
// each). Any violation fails; a screenshot of each page is kept with the report.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

const PAGES = [
  ["log", "/"],
  ["events", "/events"],
  ["event", "/events/1"],
  ["report", "/reports/1"],
  ["graphs", "/graphs"],
  ["settings", "/settings"],
  ["people", "/people"],
  ["invite", "/people/invite"],
];

async function check(page, testInfo, name) {
  await testInfo.attach(name, {
    body: await page.screenshot({ fullPage: true }),
    contentType: "image/png",
  });
  const results = await new AxeBuilder({ page }).withTags(TAGS).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
}

for (const [name, path] of PAGES) {
  test(`${name} has no accessibility violations`, async ({ page }, testInfo) => {
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error)));
    page.on("console", (message) => message.type() === "error" && errors.push(message.text()));
    await page.goto(path);
    await check(page, testInfo, name);
    expect(errors).toEqual([]);
  });
}

test.describe("signed out", () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  test("sign in has no accessibility violations", async ({ page }, testInfo) => {
    await page.goto("/login");
    await check(page, testInfo, "sign-in");
  });
});

test("the open sheets and menus have no accessibility violations", async ({ page }, testInfo) => {
  await page.goto("/");
  await page.click("a[data-dialog=rating-guide]");
  await expect(page.locator("dialog#rating-guide")).toBeVisible();
  await check(page, testInfo, "rating-sheet");
  await page.keyboard.press("Escape");
  await page.click("summary.avatar");
  await check(page, testInfo, "account-menu");
});
