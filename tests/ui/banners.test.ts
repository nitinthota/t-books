import assert from "node:assert/strict";
import { test } from "node:test";
import { CREDENTIALS_BANNER, OFFLINE_BANNER } from "../../src/lib/t-books/constants.ts";

test("UI copy used by banners never dumps technical traces", () => {
  assert.doesNotMatch(OFFLINE_BANNER, /sqlite|panic|stack/i);
  assert.doesNotMatch(CREDENTIALS_BANNER, /token|secret|password/i);
});
