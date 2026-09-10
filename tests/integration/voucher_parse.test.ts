import assert from "node:assert/strict";
import { test } from "node:test";
import { parseVoucherValues } from "../../src/lib/t-books/voucher-parse.ts";

function row(n: string, extra: Record<number, string> = {}): string[] {
  const out = [
    n,
    "2026-01-10",
    "INV",
    "VEND",
    "",
    "",
    "",
    "",
    "CUST",
    "PI",
    "2026-01-11",
    "10000",
    "40",
    "First",
    "0",
    "4000",
    "UTR",
    "2026-01-12",
    "6000",
    "",
  ];
  for (const [k, v] of Object.entries(extra)) out[Number(k)] = v;
  return out;
}

test("refresh parser skips bad voucher numbers", () => {
  const parsed = parseVoucherValues([row("20"), row("20.1"), row("ABC"), row("-3")]);
  assert.equal(parsed.vouchers.length, 1);
  assert.equal(parsed.skipped, 3);
});

test("blank and huge money do not throw", () => {
  const parsed = parseVoucherValues([
    row("21", { 11: "", 15: "not-a-number" }),
    row("22", { 11: "1000000000000" }),
  ]);
  assert.equal(parsed.vouchers.length, 2);
});
