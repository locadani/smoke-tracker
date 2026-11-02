// Starts the app on a fresh SQLite file and checks its pages at phone size, in the light and
// the dark theme. SMOKE_TRACKER_BIN names the built binary (cargo build -p smoke-tracker).
import { defineConfig, devices } from "@playwright/test";
import { BASE_URL, serverEnv, binary } from "./setup/app.js";

export default defineConfig({
  testDir: "./specs",
  globalSetup: "./setup/seed.js",
  fullyParallel: false,
  workers: 1,
  forbidOnly: Boolean(process.env.CI),
  reporter: [["list"], ["html", { open: "never", outputFolder: "report" }]],
  outputDir: "results",
  use: {
    baseURL: BASE_URL,
    viewport: { width: 390, height: 844 },
    // The bottom sheet's rise animation would be caught half-faded.
    reducedMotion: "reduce",
    storageState: ".auth/resident.json",
  },
  projects: [
    { name: "light", use: { colorScheme: "light" } },
    { name: "dark", use: { colorScheme: "dark" } },
    // Safari drops Secure cookies on http://localhost: WebKit runs only the signed-out camera check.
    ...["iPhone 15", "iPhone 15 landscape"].map((name) => ({
      name,
      testMatch: "camera-layout.spec.js",
      use: { ...devices[name], reducedMotion: "reduce" },
    })),
  ],
  webServer: {
    command: `"${binary}" serve`,
    url: `${BASE_URL}/healthz`,
    env: serverEnv,
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
