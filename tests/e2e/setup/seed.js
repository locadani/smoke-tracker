// Seeds the app through the app itself (look-and-feel spec §10): the resident comes from the
// create-resident command, which reads the password from a terminal, so it runs under a
// pseudo-terminal; everything else is typed into the pages like a person would.
import { execFileSync } from "node:child_process";
import { chromium } from "@playwright/test";
import { BASE_URL, RESIDENT, binary, serverEnv } from "./app.js";

const PTY = "import pty, sys; pty.spawn(sys.argv[1:])";

function createResident() {
  execFileSync("python3", ["-c", PTY, binary, "create-resident", RESIDENT.username], {
    input: `${RESIDENT.password}\n${RESIDENT.password}\n`,
    env: { ...process.env, ...serverEnv },
    stdio: ["pipe", "ignore", "inherit"],
  });
}

/** Ticks a room and picks its level, as on the Log screen. */
async function rate(page, room, level) {
  const line = page.locator(".room", { hasText: room });
  await line.locator(".room-check").click();
  await line.locator(`.level:has(input[value="${level}"]) > span`).first().click();
}

export default async function seed() {
  createResident();
  const browser = await chromium.launch();
  const page = await browser.newPage({ baseURL: BASE_URL });
  await page.goto("/login");
  await page.fill("input[name=username]", RESIDENT.username);
  await page.fill("input[name=password]", RESIDENT.password);
  await page.click("form[action='/login'] button[type=submit]");

  await page.goto("/");
  await rate(page, "Bedroom", 6);
  await rate(page, "Living room", 3);
  await page.locator("label.chip", { hasText: "Cough" }).click();
  await page.click("button[value=log]");
  await page.waitForURL("**/?saved=report");

  await page.fill("input[name=pm25]", "18");
  await page.click("button[value=log]");
  await page.waitForURL("**/?saved=pm25");

  await page.goto("/people/invite");
  await page.fill("input[name=username]", "visitor");
  await page.click("form.invite button[type=submit]");
  await page.waitForSelector(".qr-code");

  await page.context().storageState({ path: ".auth/resident.json" });
  await browser.close();
}
