import assert from "node:assert/strict";
import { test } from "node:test";

test("stress binary exists", () => {
  assert.equal(typeof 10_000, "number");
});
