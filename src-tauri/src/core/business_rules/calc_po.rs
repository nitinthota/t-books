//! `calcPo` — preview and save use this function only.
//! Negative qty / rates are deductions, matching loopbook `po-calc.ts`.

use super::hr_payroll::PayrollMoney;
use super::money::rupees_to_paise;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermUnit {
    Days,
    Months,
}

#[derive(Debug, Clone, Default)]
pub struct PoItemInput {
    pub item_name: String,
    pub description: String,
    pub qty: f64,
    pub unit_rate_rupees: f64,
    pub gst_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoItemCalc {
    pub item_name: String,
    pub description: String,
    pub qty: f64,
    pub unit_rate_paise: i64,
    pub gst_pct: f64,
    pub line_subtotal_paise: i64,
    pub gst_amount_paise: i64,
    pub total_cost_paise: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoSummary {
    pub payment_term_days: i64,
    pub items: Vec<PoItemCalc>,
    pub subtotal_paise: i64,
    pub gst_paise: i64,
    pub grand_total_paise: i64,
    pub amount_received_paise: i64,
    pub balance_paise: i64,
}

pub type PayrollCalc = PayrollMoney;

pub fn clamp_pct(value: f64) -> f64 {
    let n = if value.is_finite() { value } else { 0.0 };
    n.clamp(0.0, 100.0)
}

pub fn payment_term_days(value: f64, unit: TermUnit) -> i64 {
    let n = if value.is_finite() { value.max(0.0) } else { 0.0 };
    let days = match unit {
        TermUnit::Months => n * 30.0,
        TermUnit::Days => n,
    };
    days as i64
}

pub fn calc_po_item(item: &PoItemInput) -> PoItemCalc {
    let qty = if item.qty.is_finite() { item.qty } else { 0.0 };
    let gst = clamp_pct(item.gst_pct);
    let unit = rupees_to_paise(item.unit_rate_rupees);
    let line = super::money::js_round(qty * unit as f64);
    let gst_amt = super::money::js_round((line as f64 * gst) / 100.0);
    PoItemCalc {
        item_name: item.item_name.trim().to_string(),
        description: item.description.trim().to_string(),
        qty,
        unit_rate_paise: unit,
        gst_pct: gst,
        line_subtotal_paise: line,
        gst_amount_paise: gst_amt,
        total_cost_paise: line + gst_amt,
    }
}

pub fn calc_po(
    payment_term_value: f64,
    payment_term_unit: TermUnit,
    amount_received_rupees: f64,
    items: &[PoItemInput],
) -> PoSummary {
    let items: Vec<PoItemCalc> = items.iter().map(calc_po_item).collect();
    let subtotal: i64 = items.iter().map(|r| r.line_subtotal_paise).sum();
    let gst: i64 = items.iter().map(|r| r.gst_amount_paise).sum();
    let grand = subtotal + gst;
    let received = rupees_to_paise(amount_received_rupees);
    PoSummary {
        payment_term_days: payment_term_days(payment_term_value, payment_term_unit),
        items,
        subtotal_paise: subtotal,
        gst_paise: gst,
        grand_total_paise: grand,
        amount_received_paise: received,
        balance_paise: grand - received,
    }
}

pub fn calc_payroll(pf_company_rupees: f64, pf_employee_rupees: f64, tds_rupees: f64) -> PayrollCalc {
    calc_payroll_money(0.0, pf_company_rupees, pf_employee_rupees, tds_rupees)
}

/// Loopbook `calcPayroll`: salary optional (0 keeps PF-only callers compiling).
pub fn calc_payroll_money(
    salary_rupees: f64,
    pf_company_rupees: f64,
    pf_employee_rupees: f64,
    tds_rupees: f64,
) -> PayrollCalc {
    let salary = rupees_to_paise(salary_rupees);
    let company = rupees_to_paise(pf_company_rupees);
    let employee = rupees_to_paise(pf_employee_rupees);
    let tds = rupees_to_paise(tds_rupees);
    PayrollCalc {
        salary_paise: salary,
        pf_company_paise: company,
        pf_employee_paise: employee,
        pf_total_paise: company + employee,
        tds_paise: tds,
        net_paise: salary - employee - tds,
        ctc_paise: salary + company,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_terms_store_as_days() {
        assert_eq!(payment_term_days(15.0, TermUnit::Days), 15);
        assert_eq!(payment_term_days(2.0, TermUnit::Months), 60);
    }

    #[test]
    fn line_and_summary_money_is_paise() {
        let s = calc_po(
            1.0,
            TermUnit::Months,
            100.0,
            &[
                PoItemInput {
                    description: "Cable".into(),
                    qty: 2.0,
                    unit_rate_rupees: 50.0,
                    gst_pct: 18.0,
                    ..Default::default()
                },
                PoItemInput {
                    description: "Skip".into(),
                    qty: 0.0,
                    unit_rate_rupees: 10.0,
                    gst_pct: 18.0,
                    ..Default::default()
                },
            ],
        );
        assert_eq!(s.payment_term_days, 30);
        assert_eq!(s.items[0].line_subtotal_paise, 10000);
        assert_eq!(s.items[0].gst_amount_paise, 1800);
        assert_eq!(s.items[0].total_cost_paise, 11800);
        assert_eq!(s.subtotal_paise, 10000);
        assert_eq!(s.gst_paise, 1800);
        assert_eq!(s.grand_total_paise, 11800);
        assert_eq!(s.amount_received_paise, 10000);
        assert_eq!(s.balance_paise, 1800);
    }

    #[test]
    fn negative_qty_is_allowed_for_deductions() {
        let s = calc_po(
            0.0,
            TermUnit::Days,
            0.0,
            &[PoItemInput {
                description: "Credit".into(),
                qty: -1.0,
                unit_rate_rupees: 100.0,
                gst_pct: 18.0,
                ..Default::default()
            }],
        );
        assert_eq!(s.grand_total_paise, -11800);
        assert_eq!(s.balance_paise, -11800);
    }

    #[test]
    fn gst_percent_is_clamped() {
        assert_eq!(clamp_pct(118.0), 100.0);
        assert_eq!(clamp_pct(-5.0), 0.0);
    }

    #[test]
    fn payroll_pf_total_is_company_plus_employee() {
        let p = calc_payroll(1800.0, 1800.0, 500.0);
        assert_eq!(p.pf_total_paise, 360_000);
        assert_eq!(p.tds_paise, 50_000);
        assert_eq!(p.salary_paise, 0);
        assert_eq!(p.net_paise, -230_000);
        assert_eq!(p.ctc_paise, 180_000);
        let full = calc_payroll_money(50_000.0, 1800.0, 1800.0, 500.0);
        assert_eq!(full.salary_paise, 5_000_000);
        assert_eq!(full.net_paise, 5_000_000 - 180_000 - 50_000);
        assert_eq!(full.ctc_paise, 5_000_000 + 180_000);
    }
}
