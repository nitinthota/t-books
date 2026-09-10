import assert from "node:assert/strict";
import { test } from "node:test";
import { parseVoucherNumber } from "../../src/lib/t-books/business_rules/payment.ts";

test("dirty guard contract: voucher 1001 is an integer id", () => {
  assert.equal(parseVoucherNumber("1001"), 1001);
});
