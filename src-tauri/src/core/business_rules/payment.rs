//! Payments stay on the same voucher. At most five PI / payment blocks.
//! Matches loopbook `voucher-data.ts` totals: paid + TDS = payment.

use super::status::{is_na_tax_inv, payment_status, tax_flag_for, TaxFlag};

pub const MAX_PAYMENT_BLOCKS: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct PaymentBlock {
    pub slot: i32,
    pub pi_no: String,
    pub pi_date: String,
    pub pi_value_rupees: Option<f64>,
    pub payment_percent: Option<f64>,
    pub description: String,
    pub tds_rupees: Option<f64>,
    pub paid_rupees: Option<f64>,
    pub payment_details: String,
    pub payment_date: String,
    pub remaining_rupees: Option<f64>,
    pub remarks: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoucherInput {
    pub voucher_number: i64,
    pub tax_invoice: String,
    pub vendor: String,
    pub bank: String,
    pub account_number: String,
    pub ifsc: String,
    pub gst: String,
    pub project: String,
    pub comments: String,
    pub payments: Vec<PaymentBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoucherComputed {
    pub total_value: f64,
    pub total_paid: f64,
    pub remaining: f64,
    pub status: String,
    pub tax_flag: TaxFlag,
}

pub fn empty_payment(slot: i32) -> PaymentBlock {
    PaymentBlock {
        slot,
        pi_no: String::new(),
        pi_date: String::new(),
        pi_value_rupees: None,
        payment_percent: None,
        description: String::new(),
        tds_rupees: None,
        paid_rupees: None,
        payment_details: String::new(),
        payment_date: String::new(),
        remaining_rupees: None,
        remarks: String::new(),
    }
}

pub fn amount_or_zero(raw: &str) -> f64 {
    parse_amount(raw).unwrap_or(0.0)
}

pub fn parse_amount(raw: &str) -> Option<f64> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '₹' | ',' | '%' | ' ' | '\t' | '\n' | '\r'))
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || cleaned == "-" || cleaned == "\u2014" {
        return None;
    }
    let n: f64 = cleaned.parse().ok()?;
    if n.is_finite() {
        Some(n)
    } else {
        None
    }
}
