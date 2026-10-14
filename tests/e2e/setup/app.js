// Where the app under test lives and how it is configured; shared by the config and the seed.
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

export const BASE_URL = "http://localhost:8181";
export const RESIDENT = { username: "e2e", password: "a long enough passphrase" };

export const binary = resolve(
  process.env.SMOKE_TRACKER_BIN ?? join(import.meta.dirname, "../../../target/debug/smoke-tracker"),
);

const dataDir = process.env.E2E_DATA_DIR ?? mkdtempSync(join(tmpdir(), "smoke-e2e-"));
process.env.E2E_DATA_DIR = dataDir;

export const serverEnv = {
  DATABASE_URL: `sqlite://${join(dataDir, "e2e.db")}`,
  APP_BASE_URL: BASE_URL,
  APP_TIMEZONE: "Europe/Rome",
  AUTO_CLOSE_HOUR: "4",
  BIND_ADDR: "127.0.0.1:8181",
  RUST_LOG: "warn",
};
