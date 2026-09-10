import assert from "node:assert/strict";
import { test } from "node:test";
import { calcPayroll, calcPo, clampPct, paymentTermDays } from "./calc-po.ts";
import { formatInr, paiseToRupees, rupeesToPaise } from "./money.ts";
import {
  addPayment,
  canAddPayment,
  emptyPayment,
  occupiedPaymentCount,
  parseAmount,
  parseVoucherNumber,
  summarizeVoucher,
  voucher1001Fixture,
  voucherTotals,
} from "./payment.ts";
import { paymentStatus, taxFlagFor } from "./status.ts";
import { LOOPBOOK_LOGIC_VERSION } from "./index.ts";

test("loopbook logic version is v1", () => {
  assert.equal(LOOPBOOK_LOGIC_VERSION, "v1");
});

test("rupees to paise rounds", () => {
  assert.equal(rupeesToPaise(1), 100);
  assert.equal(rupeesToPaise(21.494), 2149);
  assert.equal(rupeesToPaise(Number.NaN), 0);
  assert.equal(rupeesToPaise(Number.POSITIVE_INFINITY), 0);
});

test("paise format", () => {
  assert.equal(paiseToRupees(21494), 214.94);
  assert.equal(formatInr(0), "₹0.00");
  assert.match(formatInr(10000000), /₹1,00,000/);
});

test("payment terms store as days", () => {
  assert.equal(paymentTermDays(15, "days"), 15);
  assert.equal(paymentTermDays(2, "months"), 60);
});

test("line and summary money is paise", () => {
  const s = calcPo({
    payment_term_value: 1,
    payment_term_unit: "months",
    amount_received_rupees: 100,
    items: [
      { description: "Cable", qty: 2, unit_rate_rupees: 50, gst_pct: 18 },
      { description: "Skip", qty: 0, unit_rate_rupees: 10, gst_pct: 18 },
    ],
  });
  assert.equal(s.payment_term_days, 30);
  assert.equal(s.items[0].line_subtotal_paise, 10000);
  assert.equal(s.items[0].gst_amount_paise, 1800);
  assert.equal(s.items[0].total_cost_paise, 11800);
  assert.equal(s.subtotal_paise, 10000);
  assert.equal(s.gst_paise, 1800);
  assert.equal(s.grand_total_paise, 11800);
  assert.equal(s.amount_received_paise, 10000);
  assert.equal(s.balance_paise, 1800);
});

test("negative qty is allowed for deductions", () => {
  const s = calcPo({
    payment_term_value: 0,
    payment_term_unit: "days",
    amount_received_rupees: 0,
    items: [{ description: "Credit", qty: -1, unit_rate_rupees: 100, gst_pct: 18 }],
  });
  assert.equal(s.grand_total_paise, -11800);
  assert.equal(s.balance_paise, -11800);
});

test("GST percent is clamped", () => {
  assert.equal(clampPct(118), 100);
  assert.equal(clampPct(-5), 0);
});

test("payroll PF total is company + employee", () => {
  const p = calcPayroll({ pf_company_rupees: 1800, pf_employee_rupees: 1800, tds_rupees: 500 });
  assert.equal(p.pf_total_paise, 360000);
  assert.equal(p.tds_paise, 50000);
});

test("payment status", () => {
  assert.equal(paymentStatus(100, 0, "INV"), "Pending Payment");
  assert.equal(paymentStatus(100, 40, "INV"), "Partial Payment");
  assert.equal(paymentStatus(100, 100, "INV"), "Full Payment");
  assert.equal(paymentStatus(100, 120, "INV"), "Advance Payment");
  assert.equal(paymentStatus(100, 0, ""), "Missing Tax Invoice");
});

test("tax flags", () => {
  assert.equal(taxFlagFor("", ""), "missing");
  assert.equal(taxFlagFor("NA", ""), "na_needs_comment");
  assert.equal(taxFlagFor("NA", "cash purchase"), "na_ok");
  assert.equal(taxFlagFor("INV-1", ""), "ok");
});

test("column A is a strict integer", () => {
  assert.equal(parseVoucherNumber("20"), 20);
  assert.equal(parseVoucherNumber("20.0"), 20);
  assert.equal(parseVoucherNumber("20.1"), null);
  assert.equal(parseVoucherNumber("VOUCHER_1001"), null);
});

test("VOUCHER_1001 totals and status", () => {
  const v = voucher1001Fixture();
  const c = summarizeVoucher(v);
  assert.equal(v.voucher_number, 1001);
  assert.equal(c.total_value, 100000);
  assert.equal(c.total_paid, 42000);
  assert.equal(c.remaining, 58000);
  assert.equal(c.status, "Partial Payment");
  assert.equal(c.tax_flag, "ok");
});

test("payments stay on the same voucher — max five", () => {
  let blocks = voucher1001Fixture().payments;
  assert.equal(canAddPayment(blocks), true);
  for (let i = 3; i <= 5; i++) {
    const r = addPayment(blocks, { ...emptyPayment(i), paid_rupees: 1 });
    assert.equal(r.ok, true);
    if (r.ok) blocks = r.payments;
  }
  assert.equal(occupiedPaymentCount(blocks), 5);
  assert.equal(canAddPayment(blocks), false);
  const err = addPayment(blocks, emptyPayment(6));
  assert.equal(err.ok, false);
});

test("TDS counts as payment", () => {
  const t = voucherTotals([
    {
      ...emptyPayment(1),
      pi_value_rupees: 5000,
      paid_rupees: 4500,
      tds_rupees: 500,
    },
  ]);
  assert.equal(t.total_value, 5000);
  assert.equal(t.total_paid, 5000);
  assert.equal(t.remaining, 0);
});

test("blank and odd cells coerce", () => {
  assert.equal(parseAmount(""), null);
  assert.equal(parseAmount("-"), null);
  assert.equal(parseAmount("₹1,200.50"), 1200.5);
});
