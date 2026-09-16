import assert from "node:assert/strict";
import { test } from "node:test";
import { calcPayroll, calcPo, clampPct, paymentTermDays } from "./calc-po.ts";
import { formatInr, formatQty, paiseToRupees, rupeesToPaise } from "./money.ts";
import {
  addPayment,
  canAddPayment,
  emptyPayment,
  isDottedChildSerial,
  isDummySerial,
  occupiedPaymentCount,
  parseAmount,
  parseVoucherNumber,
  shouldSkipSheetRow,
  summarizeVoucher,
  voucher1001Fixture,
  voucherTotals,
} from "./payment.ts";
import { paymentStatus, taxFlagFor } from "./status.ts";
import { LOOPBOOK_LOGIC_VERSION } from "./index.ts";
import { currentIndianFy, fyBounds, trialBalanced, trialClosing } from "./fy.ts";
import {
  calcSalarySlip,
  keepPostedPayNumber,
  linesBalance,
  salaryPeriodTaken,
  salaryVoucherLines,
} from "./hr-payroll.ts";
import { allocateVoucherSerial, nextPaymentNumber, nextPurchaseNumber, nextSalaryNumber } from "./alloc.ts";
import { classifyPurchasePayment, isLegacyPayNumber, paymentOrdinal } from "./purchase-status.ts";
import { MemoryHive, hiveConflictMessage } from "../hive.ts";
import { canMutate } from "./rbac.ts";

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
  assert.equal(formatQty(Number.NaN), "—");
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

test("payroll PF total is company + employee; salary optional", () => {
  const p = calcPayroll({ pf_company_rupees: 1800, pf_employee_rupees: 1800, tds_rupees: 500 });
  assert.equal(p.pf_total_paise, 360000);
  assert.equal(p.tds_paise, 50000);
  assert.equal(p.salary_paise, 0);
  const full = calcPayroll({
    salary_rupees: 50000,
    pf_company_rupees: 1800,
    pf_employee_rupees: 1800,
    tds_rupees: 500,
  });
  assert.equal(full.net_paise, 50000 * 100 - 1800 * 100 - 500 * 100);
  assert.equal(full.ctc_paise, 50000 * 100 + 1800 * 100);
});

test("payment status rounds each side to paise", () => {
  assert.equal(paymentStatus(100, 0, "INV"), "Pending Payment");
  assert.equal(paymentStatus(100, 40, "INV"), "Partial Payment");
  assert.equal(paymentStatus(100, 100, "INV"), "Full Payment");
  assert.equal(paymentStatus(100, 120, "INV"), "Advance Payment");
  assert.equal(paymentStatus(100, 0, ""), "Missing Tax Invoice");
  assert.equal(paymentStatus(10.004, 10.006, "INV"), "Advance Payment");
  assert.equal(paymentStatus(10.006, 10.004, "INV"), "Partial Payment");
});

test("tax flags", () => {
  assert.equal(taxFlagFor("", ""), "missing");
  assert.equal(taxFlagFor("NA", ""), "na_needs_comment");
  assert.equal(taxFlagFor("NA", "cash purchase"), "na_ok");
  assert.equal(taxFlagFor("INV-1", ""), "ok");
});

test("column A is a strict integer; 20.1 is dotted child", () => {
  assert.equal(parseVoucherNumber("20"), 20);
  assert.equal(parseVoucherNumber("20.0"), 20);
  assert.equal(parseVoucherNumber("20.1"), null);
  assert.equal(parseVoucherNumber("VOUCHER_1001"), null);
  assert.equal(isDottedChildSerial("20.1"), true);
  assert.equal(isDottedChildSerial("20.0"), false);
  assert.equal(isDummySerial("VOUCHER_1001"), true);
  assert.equal(isDummySerial("PUR-0001"), false);
  assert.equal(shouldSkipSheetRow("20", ""), true);
  assert.equal(shouldSkipSheetRow("20.1", "VEND_02"), true);
  assert.equal(shouldSkipSheetRow("20", "VEND_02"), false);
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

test("Indian FY is 1 Apr–31 Mar and trial balances", () => {
  assert.equal(currentIndianFy(new Date("2026-04-01T00:00:00Z")), "2026-27");
  assert.equal(currentIndianFy(new Date("2026-03-31T00:00:00Z")), "2025-26");
  assert.deepEqual(fyBounds("2026-27"), { start: "2026-04-01", end: "2027-03-31" });
  assert.equal(trialClosing(100, 40, 25), 115);
  assert.equal(trialBalanced([{ debit_paise: 100, credit_paise: 0 }, { debit_paise: 0, credit_paise: 100 }]), true);
});

test("SAL-n slip balances; posted number is sacred", () => {
  const slip = calcSalarySlip({
    kind: "salary",
    salary_paise: 5_000_000,
    pf_employee_paise: 180_000,
    pf_company_paise: 180_000,
    tds_paise: 50_000,
    recovery_paise: 200_000,
  });
  assert.equal(slip.net_paise, 5_000_000 - 180_000 - 50_000 - 200_000);
  assert.equal(linesBalance(salaryVoucherLines(slip, "bank")), true);
  assert.equal(keepPostedPayNumber("SAL-0001", "SAL-0009"), "SAL-0001");
  assert.equal(
    salaryPeriodTaken(
      [{ employee_id: "p1", employee_name: "CUST_01", pay_kind: "salary", period_month: 9, period_year: 2026, id: "a" }],
      { employee_id: "p1", employee_name: "CUST_01", period_month: 9, period_year: 2026, id: "b" },
    ),
    true,
  );
});

test("PUR/PAY allocate from hive list; dummy rows never allocate", () => {
  assert.equal(nextPurchaseNumber(["VOUCHER_1001", "PUR-0001", "PURC:0003"]), "PUR-0004");
  assert.equal(nextPaymentNumber(["PAY-0001", "CUST_01"]), "PAY-0002");
  assert.equal(nextSalaryNumber(["SAL-0001"]), "SAL-0002");
  assert.equal(allocateVoucherSerial(["VCH-0004~id", "VOUCHER_1001"], "VCH"), "VCH-0005");
  assert.equal(isLegacyPayNumber("PAY-0001"), false);
  const { payClass } = classifyPurchasePayment({
    goodsReceived: false,
    taxInvoiceMissing: true,
    grandPaise: 100_000,
    paidBeforePaise: 0,
    thisPaise: 10_000,
  });
  assert.equal(payClass, "advance");
});

test("1st payment is the ordinal on that bill, not the PAY serial", () => {
  assert.equal(paymentOrdinal(1), "1st payment");
  assert.equal(paymentOrdinal(2), "2nd payment");
  assert.equal(paymentOrdinal(3), "3rd payment");
  assert.equal(paymentOrdinal(10), "10th payment");
  assert.equal(paymentOrdinal(11), "11th payment");
  assert.equal(paymentOrdinal(21), "21st payment");
});

test("viewer cannot mutate", () => {
  assert.equal(canMutate("viewer"), false);
  assert.equal(canMutate("operator"), true);
  assert.equal(canMutate("owner"), true);
});

test("hive CAS is fp+rev and never silent overwrite", () => {
  const hive = new MemoryHive();
  hive.ensureTab("voucher", ["serial_no"]);
  const first = hive.casRow("voucher", "20", "", 0, ["20"], "aaa");
  assert.equal(first.kind, "ok");
  const conflict = hive.casRow("voucher", "20", "stale", 0, ["20"], "bbb");
  assert.equal(conflict.kind, "conflict");
  if (conflict.kind === "conflict") {
    assert.equal(conflict.message, hiveConflictMessage("20"));
  }
  assert.equal(hive.getRow("voucher", "20")?.fp, "aaa");
});

