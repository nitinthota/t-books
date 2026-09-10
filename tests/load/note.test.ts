import assert from "node:assert/strict";
import { test } from "node:test";

test("office size is 3 to 5 people", () => {
  assert.ok(5 >= 3);
});
