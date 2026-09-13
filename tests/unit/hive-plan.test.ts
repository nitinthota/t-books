import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { tabName } from "../../src/lib/t-books/hive.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");

test("hive-map keeps the raw sheet read-only and off the live voucher tab", () => {
  const raw = readFileSync(join(root, "src-tauri/hive-map.json"), "utf8");
  const map = JSON.parse(raw) as {
    rawSource: { spreadsheetId: string; tab: string; write: boolean };
    workbooks: Array<{ kind: string; id: string; tab: string }>;
  };
  assert.equal(map.rawSource.write, false);
  assert.equal(map.rawSource.spreadsheetId, "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE");
  const voucher = map.workbooks.find((w) => w.kind === "voucher");
  assert.ok(voucher);
  assert.notEqual(voucher.id, map.rawSource.spreadsheetId);
  assert.equal(voucher.tab, "Voucher register");
  const access = map.workbooks.find((w) => w.kind === "access");
  assert.ok(access);
  assert.equal(access.tab, "Access");
  assert.notEqual(access.id, map.rawSource.spreadsheetId);
  assert.equal(tabName("voucher"), "Voucher register");
  assert.equal(tabName("payment"), "Purchase payments");
});

test("example service account file has no real PEM", () => {
  const example = readFileSync(join(root, "secrets/google-service-account.example.json"), "utf8");
  assert.match(example, /paste PEM here/i);
  assert.doesNotMatch(example, /BEGIN PRIVATE KEY/);
});

test("migration log and credentials extra tabs sit on Access workbook", () => {
  const raw = readFileSync(join(root, "src-tauri/hive-map.json"), "utf8");
  const map = JSON.parse(raw) as {
    extraTabs: Array<{ workbookKey: string; tab: string; kind: string }>;
  };
  const log = map.extraTabs.find((t) => t.kind === "migration_log");
  assert.ok(log);
  assert.equal(log.tab, "Migration_Log");
  assert.equal(log.workbookKey, "access");
  const creds = map.extraTabs.find((t) => t.kind === "credentials");
  assert.ok(creds);
  assert.equal(creds.tab, "Credentials");
});
