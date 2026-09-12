import assert from "node:assert/strict";
import { test } from "node:test";
import {
  conflictMessage,
  sha256Hex,
  voucherFingerprint,
} from "../../src/lib/t-books/business_rules/fingerprint.ts";
import { voucher1001Fixture } from "../../src/lib/t-books/business_rules/payment.ts";

test("sha256 abc matches FIPS", () => {
  assert.equal(
    sha256Hex("abc"),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
});

test("same voucher same fingerprint", () => {
  const v = voucher1001Fixture();
  const row = {
    voucher_number: v.voucher_number,
    voucher_date: "",
    tax_invoice: v.tax_invoice,
    vendor: v.vendor,
    bank: v.bank,
    account_number: v.account_number,
    ifsc: v.ifsc,
    gst: v.gst,
    project: v.project,
    payments: v.payments,
  };
  assert.equal(voucherFingerprint(row), voucherFingerprint(row));
  assert.equal(voucherFingerprint(row).length, 64);
});

test("conflict copy is exact", () => {
  assert.equal(
    conflictMessage(1021),
    "Voucher 1021 was just updated by another user. Reload and submit again.",
  );
  assert.equal(
    conflictMessage("PAY-0001"),
    "PAY-0001 was just updated by another user. Reload and submit again.",
  );
});
