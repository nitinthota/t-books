import assert from "node:assert/strict";
import { test } from "node:test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { nsisBundleDirs, pickSetupExe } from "./rename-installer.mjs";

test("picks Tauri NSIS -setup.exe, not WebView2", () => {
  assert.equal(
    pickSetupExe([
      "MicrosoftEdgeWebview2Setup.exe",
      "T Books_1.0.0_x64-setup.exe",
      "nsis-output.exe",
    ]),
    "T Books_1.0.0_x64-setup.exe",
  );
});

test("refuses credentials-named and webview files", () => {
  assert.equal(pickSetupExe(["credentials-setup.exe", "MicrosoftEdgeWebview2Setup.exe"]), null);
});

test("nsis dirs include default and msvc triple paths", () => {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const dirs = nsisBundleDirs(root);
  assert.match(dirs[0], /target[/\\]release[/\\]bundle[/\\]nsis$/);
  assert.match(dirs[1], /x86_64-pc-windows-msvc[/\\]release[/\\]bundle[/\\]nsis$/);
});
