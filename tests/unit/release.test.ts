import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { LOOPBOOK_NAV } from "../../src/lib/t-books/modules.ts";
import { canMutate } from "../../src/lib/t-books/business_rules/rbac.ts";
import {
  amountOrZero,
  isDummySerial,
  parseVoucherNumber,
  shouldSkipSheetRow,
} from "../../src/lib/t-books/business_rules/payment.ts";
import { OWNER_EMAIL } from "../../src/lib/t-books/constants.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");

test("release plan names every Loopbook module", () => {
  const plan = readFileSync(join(root, "docs/RELEASE_TEST.md"), "utf8");
  for (const row of LOOPBOOK_NAV) {
    assert.match(plan, new RegExp(row.label));
  }
  assert.match(plan, /Voucher_Raw_Data/);
  assert.match(plan, /read-only/i);
  assert.doesNotMatch(plan, /BEGIN PRIVATE KEY/);
});

test("Windows NSIS packaging is per-user T Books with no updater bundle", () => {
  const conf = readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8");
  const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
  const hooks = readFileSync(join(root, "src-tauri/windows/hooks.nsh"), "utf8");
  const ignore = readFileSync(join(root, ".gitignore"), "utf8");
  const rename = readFileSync(join(root, "scripts/rename-installer.mjs"), "utf8");
  const release = readFileSync(join(root, "docs/RELEASE.md"), "utf8");
  const workflow = readFileSync(join(root, ".github/workflows/build.yml"), "utf8");
  const pkg = readFileSync(join(root, "package.json"), "utf8");
  const rustLib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
  const wrapper = readFileSync(join(root, "scripts/with-app-env.mjs"), "utf8");
  assert.match(conf, /"productName": "T Books"/);
  assert.match(conf, /"mainBinaryName": "t-books"/);
  assert.match(conf, /"version": "1.0.0"/);
  assert.match(conf, /"identifier": "com.tbooks.desktop"/);
  assert.match(conf, /"targets": \["nsis"\]/);
  assert.match(conf, /"createUpdaterArtifacts": false/);
  assert.match(conf, /"installMode": "currentUser"/);
  assert.match(conf, /"startMenuFolder": "T Books"/);
  assert.match(conf, /embedBootstrapper/);
  assert.doesNotMatch(conf, /"resources"/);
  assert.doesNotMatch(conf, /credentials\.json/);
  assert.doesNotMatch(cargo, /tauri-plugin-updater/);
  assert.match(cargo, /version = "1.0.0"/);
  assert.match(cargo, /default-run = "t-books"/);
  assert.match(hooks, /leave %LOCALAPPDATA%\\T-Books/);
  assert.doesNotMatch(hooks, /RMDir.*T-Books/i);
  assert.match(ignore, /credentials\.json/);
  assert.match(ignore, /\*\.pem/);
  assert.match(ignore, /\*\.exe/);
  assert.match(rename, /T-Books-Setup\.exe/);
  assert.match(pkg, /"tauri:build": "tauri build && node scripts\/rename-installer\.mjs"/);
  assert.match(wrapper, /vite\/bin\/vite\.js/);
  assert.match(rustLib, /DATA_FOLDER_NAME: &str = "T-Books"/);
  assert.match(rustLib, /SCHEMA_VERSION: &str = "10"/);
  assert.match(release, /npm run tauri:build/);
  assert.match(release, /cannot.*Windows NSIS/i);
  assert.match(release, /Uninstall.*does \*\*not\*\* delete/);
  assert.match(workflow, /windows-latest/);
  assert.match(workflow, /npm run tauri:build/);
  assert.match(workflow, /dist\/installers\/T-Books-Setup\.exe/);
  assert.match(rustLib, /use tauri::Manager;/);
});

test("dummy tokens and invalid A stay out of books", () => {
  assert.equal(OWNER_EMAIL, "thotanitin123@gmail.com");
  assert.equal(isDummySerial("VOUCHER_1001"), true);
  assert.equal(isDummySerial("CUST_01"), true);
  assert.equal(isDummySerial("VEND_02"), true);
  assert.equal(parseVoucherNumber("20.1"), null);
  assert.equal(shouldSkipSheetRow("20", ""), true);
  assert.equal(amountOrZero("odd"), 0);
  assert.equal(canMutate("viewer"), false);
});
