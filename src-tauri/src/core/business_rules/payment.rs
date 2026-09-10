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
    if cleaned.is_empty() || cleaned == "-" || cleaned == "—" {
        return None;
    }
    let n: f64 = cleaned.parse().ok()?;
    if n.is_finite() {
        Some(n)
    } else {
        None
    }
}

/// Column A: `20.0` → `20`. `20.1` stays invalid as a voucher number.
pub fn normalize_voucher_no(raw: &str) -> String {
    let s = raw.trim();
    if looks_like_trailing_zero_decimal(s) {
        if let Ok(n) = s.parse::<f64>() {
            return format!("{}", n as i64);
        }
    }
    s.to_string()
}

fn looks_like_trailing_zero_decimal(s: &str) -> bool {
    let mut parts = s.split('.');
    let Some(head) = parts.next() else {
        return false;
    };
    let Some(frac) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && !head.is_empty()
        && head.chars().all(|c| c.is_ascii_digit())
        && !frac.is_empty()
        && frac.chars().all(|c| c == '0')
}

pub fn parse_voucher_number(raw: &str) -> Option<i64> {
    let n = normalize_voucher_no(raw);
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    n.parse().ok()
}

pub fn occupied_payment_count(blocks: &[PaymentBlock]) -> usize {
    blocks.iter().filter(|b| block_occupied(b)).count()
}

fn block_occupied(slot: &PaymentBlock) -> bool {
    slot.pi_value_rupees.is_some()
        || slot.paid_rupees.is_some()
        || slot.tds_rupees.is_some()
        || !slot.payment_date.trim().is_empty()
        || !slot.payment_details.trim().is_empty()
        || !slot.pi_no.trim().is_empty()
}

pub fn can_add_payment(blocks: &[PaymentBlock]) -> bool {
    occupied_payment_count(blocks) < MAX_PAYMENT_BLOCKS
}

pub fn add_payment(blocks: &[PaymentBlock], next: PaymentBlock) -> Result<Vec<PaymentBlock>, String> {
    if !can_add_payment(blocks) {
        return Err("This voucher already has 5 payments.".into());
    }
    let mut out = blocks.to_vec();
    let mut block = next;
    if block.slot < 1 || block.slot > MAX_PAYMENT_BLOCKS as i32 {
        block.slot = (occupied_payment_count(&out) as i32) + 1;
    }
    out.push(block);
    Ok(out)
}

pub fn voucher_totals(blocks: &[PaymentBlock]) -> (f64, f64, f64) {
    let mut inv_sum = 0.0;
    let mut tds_sum = 0.0;
    let mut paid_sum = 0.0;
    for slot in blocks {
        inv_sum += slot.pi_value_rupees.unwrap_or(0.0);
        tds_sum += slot.tds_rupees.unwrap_or(0.0);
        paid_sum += slot.paid_rupees.unwrap_or(0.0);
    }
    let payment = paid_sum + tds_sum;
    let remaining = super::money::js_round(inv_sum - payment) as f64;
    (inv_sum, payment, remaining)
}

pub fn summarize_voucher(voucher: &VoucherInput) -> VoucherComputed {
    let (total_value, total_paid, remaining) = voucher_totals(&voucher.payments);
    let tax_for_status = if is_na_tax_inv(&voucher.tax_invoice) {
        "NA"
    } else {
        voucher.tax_invoice.as_str()
    };
    let comments = if voucher.comments.trim().is_empty() {
        voucher
            .payments
            .iter()
            .map(|p| p.remarks.as_str())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    } else {
        voucher.comments.clone()
    };
    VoucherComputed {
        total_value,
        total_paid,
        remaining,
        status: payment_status(total_value, total_paid, tax_for_status),
        tax_flag: tax_flag_for(&voucher.tax_invoice, &comments),
    }
}

/// Dummy fixture. Never loaded as live office data.
pub fn voucher_1001_fixture() -> VoucherInput {
    VoucherInput {
        voucher_number: 1001,
        tax_invoice: "INV-1001".into(),
        vendor: "VEND_02".into(),
        bank: "BANK_01".into(),
        account_number: "00000000".into(),
        ifsc: "IFSC0000001".into(),
        gst: "GST00DUMMY".into(),
        project: "CUST_01".into(),
        comments: String::new(),
        payments: vec![
            PaymentBlock {
                slot: 1,
                pi_no: "PI-A".into(),
                pi_date: "2026-01-10".into(),
                pi_value_rupees: Some(60_000.0),
                payment_percent: Some(50.0),
                description: "First block".into(),
                tds_rupees: Some(2_000.0),
                paid_rupees: Some(28_000.0),
                payment_details: "UTR1".into(),
                payment_date: "2026-01-12".into(),
                remaining_rupees: None,
                remarks: String::new(),
            },
            PaymentBlock {
                slot: 2,
                pi_no: "PI-B".into(),
                pi_date: "2026-01-20".into(),
                pi_value_rupees: Some(40_000.0),
                payment_percent: Some(30.0),
                description: "Second block".into(),
                tds_rupees: None,
                paid_rupees: Some(12_000.0),
                payment_details: "UTR2".into(),
                payment_date: "2026-01-22".into(),
                remaining_rupees: None,
                remarks: String::new(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_a_is_a_strict_integer() {
        assert_eq!(parse_voucher_number("20"), Some(20));
        assert_eq!(parse_voucher_number("20.0"), Some(20));
        assert_eq!(parse_voucher_number(" 20.000 "), Some(20));
        assert_eq!(parse_voucher_number("20.1"), None);
        assert_eq!(parse_voucher_number("VOUCHER_1001"), None);
    }

    #[test]
    fn voucher_1001_totals_and_status() {
        let v = voucher_1001_fixture();
        let c = summarize_voucher(&v);
        assert_eq!(v.voucher_number, 1001);
        assert_eq!(c.total_value, 100_000.0);
        assert_eq!(c.total_paid, 42_000.0);
        assert_eq!(c.remaining, 58_000.0);
        assert_eq!(c.status, "Partial Payment");
        assert_eq!(c.tax_flag, TaxFlag::Ok);
    }

    #[test]
    fn payments_stay_on_the_same_voucher_max_five() {
        let mut blocks = voucher_1001_fixture().payments;
        assert!(can_add_payment(&blocks));
        for i in 3..=5 {
            blocks = add_payment(
                &blocks,
                PaymentBlock {
                    slot: i,
                    paid_rupees: Some(1.0),
                    ..empty_payment(i)
                },
            )
            .unwrap();
        }
        assert_eq!(occupied_payment_count(&blocks), 5);
        assert!(!can_add_payment(&blocks));
        let err = add_payment(&blocks, empty_payment(6)).unwrap_err();
        assert!(err.contains("5 payments"));
    }

    #[test]
    fn blank_and_odd_cells_coerce_to_none() {
        assert_eq!(parse_amount(""), None);
        assert_eq!(parse_amount("-"), None);
        assert_eq!(parse_amount("—"), None);
        assert_eq!(parse_amount("₹1,200.50"), Some(1200.50));
        assert_eq!(parse_amount("not-a-number"), None);
    }

    #[test]
    fn tds_counts_as_payment() {
        let blocks = [PaymentBlock {
            tds_rupees: Some(500.0),
            paid_rupees: Some(4_500.0),
            pi_value_rupees: Some(5_000.0),
            ..empty_payment(1)
        }];
        let (value, paid, remaining) = voucher_totals(&blocks);
        assert_eq!(value, 5_000.0);
        assert_eq!(paid, 5_000.0);
        assert_eq!(remaining, 0.0);
    }
}
