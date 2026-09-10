import assert from "node:assert/strict";
import { test } from "node:test";
import { parseVoucherValues } from "../../src/lib/t-books/voucher-parse.ts";

test("invalid dates do not crash parse", () => {
  const row = [
    "40",
    "not-a-date",
    "INV",
    "VEND",
    "",
    "",
    "",
    "",
    "CUST",
    "PI",
    "",
    "100",
    "0",
    "",
    "0",
    "0",
    "",
    "",
    "0",
    "",
  ];
  const parsed = parseVoucherValues([row]);
  assert.equal(parsed.vouchers.length, 1);
  assert.equal(parsed.vouchers[0].voucher_date, "not-a-date");
});
