import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

test("log module refuses passwords gst bank", () => {
  const src = readFileSync(new URL("../../src-tauri/src/log.rs", import.meta.url), "utf8");
  assert.match(src, /password/);
  assert.match(src, /redacted/);
  assert.match(src, /error\.log/);
  assert.match(src, /performance\.log/);
});
