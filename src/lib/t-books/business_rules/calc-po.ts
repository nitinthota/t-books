/** calcPo — preview and save use this function only. Negatives are deductions. */

import { rupeesToPaise } from "./money.ts";

export type TermUnit = "days" | "months";

export type PoItemInput = {
  description: string;
  qty: number;
  unit_rate_rupees: number;
  gst_pct: number;
};

export type PoItemCalc = {
  description: string;
  qty: number;
  unit_rate_paise: number;
  gst_pct: number;
  line_subtotal_paise: number;
  gst_amount_paise: number;
  total_cost_paise: number;
};

export type PoSummary = {
  payment_term_days: number;
  items: PoItemCalc[];
  subtotal_paise: number;
  gst_paise: number;
  grand_total_paise: number;
  amount_received_paise: number;
  balance_paise: number;
};

export function clampPct(value: number): number {
  const n = Number.isFinite(value) ? value : 0;
  return Math.min(100, Math.max(0, n));
}

export function paymentTermDays(value: number, unit: TermUnit): number {
  const n = Number.isFinite(value) ? Math.max(0, value) : 0;
  return unit === "months" ? n * 30 : n;
}

export function calcPoItem(item: PoItemInput): PoItemCalc {
  const qty = Number.isFinite(item.qty) ? item.qty : 0;
  const gst = clampPct(item.gst_pct);
  const unit = rupeesToPaise(item.unit_rate_rupees);
  const line = Math.round(qty * unit);
  const gstAmt = Math.round((line * gst) / 100);
  return {
    description: (item.description ?? "").trim(),
    qty,
    unit_rate_paise: unit,
    gst_pct: gst,
    line_subtotal_paise: line,
    gst_amount_paise: gstAmt,
    total_cost_paise: line + gstAmt,
  };
}

export function calcPo(input: {
  payment_term_value: number;
  payment_term_unit: TermUnit;
  amount_received_rupees: number;
  items: PoItemInput[];
}): PoSummary {
  const items = (input.items ?? []).map(calcPoItem);
  const subtotal = items.reduce((a, r) => a + r.line_subtotal_paise, 0);
  const gst = items.reduce((a, r) => a + r.gst_amount_paise, 0);
  const grand = subtotal + gst;
  const received = rupeesToPaise(input.amount_received_rupees);
  return {
    payment_term_days: paymentTermDays(input.payment_term_value, input.payment_term_unit),
    items,
    subtotal_paise: subtotal,
    gst_paise: gst,
    grand_total_paise: grand,
    amount_received_paise: received,
    balance_paise: grand - received,
  };
}

export function calcPayroll(input: {
  pf_company_rupees: number;
  pf_employee_rupees: number;
  tds_rupees: number;
}) {
  const company = rupeesToPaise(input.pf_company_rupees);
  const employee = rupeesToPaise(input.pf_employee_rupees);
  return {
    pf_company_paise: company,
    pf_employee_paise: employee,
    pf_total_paise: company + employee,
    tds_paise: rupeesToPaise(input.tds_rupees),
  };
}
