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
