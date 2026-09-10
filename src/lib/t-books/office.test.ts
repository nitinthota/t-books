import assert from "node:assert/strict";
import { test } from "node:test";
import { calcPayroll, calcPo, paiseToRupees } from "./business_rules/index.ts";

test("sales/purchase totals use calcPo — negatives are deductions", () => {
  const preview = calcPo({
    payment_term_value: 0,
    payment_term_unit: "days",
    amount_received_rupees: 0,
    items: [
      { description: "Cable", qty: 2, unit_rate_rupees: 50, gst_pct: 18 },
      { description: "Credit", qty: -1, unit_rate_rupees: 20, gst_pct: 18 },
    ],
  });
  assert.equal(paiseToRupees(preview.items[0].total_cost_paise), 118);
  assert.equal(Math.round(paiseToRupees(preview.items[1].total_cost_paise) * 100), -2360);
  assert.equal(preview.grand_total_paise, 9440);
});

test("payroll PF total is employee plus company via calcPayroll", () => {
  const p = calcPayroll({ pf_company_rupees: 1800, pf_employee_rupees: 1800, tds_rupees: 500 });
  assert.equal(p.pf_total_paise, 360_000);
});
