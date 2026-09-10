import assert from "node:assert/strict";
import { test } from "node:test";

test("preview still serves T Books", async () => {
  const res = await fetch("http://127.0.0.1:8080/");
  assert.equal(res.ok, true);
  const html = await res.text();
  assert.match(html, /T Books/);
});
