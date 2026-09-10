import assert from "node:assert/strict";
import { test } from "node:test";
import { parseVoucherValues } from "./voucher-parse.ts";

function row20(overrides: Partial<{ a: string; value: string; paid: string }> = {}): string[] {
  const row = [
    overrides.a ?? "20",
    "2026-01-10",
    "INV-20",
    "VEND_02",
    "BANK_01",
    "00000000",
    "IFSC0000001",
    "GST00DUMMY",
    "CUST_01",
    "PI-A",
    "2026-01-11",
    overrides.value ?? "10000",
    "40",
    "First",
    "0",
    overrides.paid ?? "4000",
    "UTR1",
    "2026-01-12",
    "6000",
    "",
  ];
  return row;
}

test("column A must be an integer", () => {
  const parsed = parseVoucherValues([
    ["voucher_number", "date", "tax inv", "vendor"],
    row20(),
    row20({ a: "20.1" }),
    row20({ a: "VOUCHER_1001" }),
  ]);
  assert.equal(parsed.vouchers.length, 1);
  assert.equal(parsed.vouchers[0].voucher_number, 20);
  assert.equal(parsed.vouchers[0].tax_invoice, "INV-20");
  assert.equal(parsed.vouchers[0].vendor, "VEND_02");
  assert.equal(parsed.vouchers[0].bank, "BANK_01");
  assert.equal(parsed.vouchers[0].project, "CUST_01");
  assert.equal(parsed.vouchers[0].payments[0].pi_no, "PI-A");
  assert.equal(parsed.skipped, 2);
  assert.ok(parsed.errors.every((e) => e.errorType === "invalid_voucher_number"));
});

test("blank and invalid numeric cells become 0", () => {
  const parsed = parseVoucherValues([row20({ value: "nope" })]);
  assert.equal(parsed.vouchers[0].payments[0].pi_value_rupees, 0);
});

test("ignores payment blocks beyond 5", () => {
  const row = row20();
  for (let s = 1; s <= 6; s++) {
    row.push(`PI-${s}`, "2026-01-11", "1", "0", "", "0", "0", "", "", "0", "");
  }
  const parsed = parseVoucherValues([row]);
  assert.equal(parsed.vouchers[0].payments.length, 5);
});

test("empty sheet is a warning path not a crash", () => {
  const parsed = parseVoucherValues([]);
  assert.equal(parsed.vouchers.length, 0);
  assert.equal(parsed.skipped, 0);
});
