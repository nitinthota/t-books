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
  assert.equal(tabName("voucher"), "Voucher_Raw_Data");
});

test("example service account file has no real PEM", () => {
  const example = readFileSync(join(root, "secrets/google-service-account.example.json"), "utf8");
  assert.match(example, /paste PEM here/i);
  assert.doesNotMatch(example, /BEGIN PRIVATE KEY/);
});
