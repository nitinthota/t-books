import assert from "node:assert/strict";
import { test } from "node:test";
import {
  CREDENTIALS_BANNER,
  OFFLINE_BANNER,
  OWNER_EMAIL,
} from "../../src/lib/t-books/constants.ts";
import { parseVoucherNumber } from "../../src/lib/t-books/business_rules/payment.ts";

test("offline banner text is exact", () => {
  assert.equal(OFFLINE_BANNER, "Offline — working on this PC. Access list and Refresh paused.");
});

test("credentials banner text is exact", () => {
  assert.equal(CREDENTIALS_BANNER, "Google credentials not found. Running offline.");
});

test("hardcoded owner email is fixed", () => {
  assert.equal(OWNER_EMAIL, "thotanitin123@gmail.com");
});

test("column A is a strict integer", () => {
  assert.equal(parseVoucherNumber("20"), 20);
  assert.equal(parseVoucherNumber("20.1"), null);
  assert.equal(parseVoucherNumber("ABC"), null);
  assert.equal(parseVoucherNumber("-9"), null);
});
