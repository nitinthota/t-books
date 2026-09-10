//! Deterministic SHA-256 of a voucher row + 5 payment blocks.
//! Same normalized input → same hash. Never includes amounts in logs.

use sha2::{Digest, Sha256};

use super::payment::{empty_payment, PaymentBlock, MAX_PAYMENT_BLOCKS};

const FINGERPRINT_VERSION: &str = "v1";

pub fn canon_num(value: Option<f64>) -> String {
    let n = value.filter(|v| v.is_finite()).unwrap_or(0.0);
    if n == 0.0 {
        return "0".into();
    }
    let rounded = (n * 1_000_000.0).round() / 1_000_000.0;
    if rounded == 0.0 {
        return "0".into();
    }
    let s = format!("{rounded:.6}");
    s.trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn pad_payments(payments: &[PaymentBlock]) -> Vec<PaymentBlock> {
    let mut out: Vec<PaymentBlock> = (1..=MAX_PAYMENT_BLOCKS as i32)
        .map(empty_payment)
        .collect();
    for block in payments.iter().take(MAX_PAYMENT_BLOCKS) {
        let idx = (block.slot.clamp(1, MAX_PAYMENT_BLOCKS as i32) as usize) - 1;
        out[idx] = block.clone();
        out[idx].slot = (idx as i32) + 1;
    }
    out
}

pub fn voucher_canonical(
    voucher_number: i64,
    voucher_date: &str,
    tax_invoice: &str,
    vendor: &str,
    bank: &str,
    account_number: &str,
    ifsc: &str,
    gst: &str,
    project: &str,
    payments: &[PaymentBlock],
) -> String {
    let mut lines = Vec::with_capacity(8 + MAX_PAYMENT_BLOCKS * 12);
    lines.push(FINGERPRINT_VERSION.to_string());
    lines.push(voucher_number.to_string());
    lines.push(voucher_date.trim().to_string());
    lines.push(tax_invoice.trim().to_string());
    lines.push(vendor.trim().to_string());
    lines.push(bank.trim().to_string());
    lines.push(account_number.trim().to_string());
    lines.push(ifsc.trim().to_string());
    lines.push(gst.trim().to_string());
    lines.push(project.trim().to_string());
    for block in pad_payments(payments) {
        lines.push(format!("--{}", block.slot));
        lines.push(block.pi_no.trim().to_string());
        lines.push(block.pi_date.trim().to_string());
        lines.push(canon_num(block.pi_value_rupees));
        lines.push(canon_num(block.payment_percent));
        lines.push(block.description.trim().to_string());
        lines.push(canon_num(block.tds_rupees));
        lines.push(canon_num(block.paid_rupees));
        lines.push(block.payment_details.trim().to_string());
        lines.push(block.payment_date.trim().to_string());
        lines.push(canon_num(block.remaining_rupees));
        lines.push(block.remarks.trim().to_string());
    }
    lines.join("\n")
}

pub fn sha256_hex(canonical: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let out = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in out {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

pub fn voucher_fingerprint(
    voucher_number: i64,
    voucher_date: &str,
    tax_invoice: &str,
    vendor: &str,
    bank: &str,
    account_number: &str,
    ifsc: &str,
    gst: &str,
    project: &str,
    payments: &[PaymentBlock],
) -> String {
    sha256_hex(&voucher_canonical(
        voucher_number,
        voucher_date,
        tax_invoice,
        vendor,
        bank,
        account_number,
        ifsc,
        gst,
        project,
        payments,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::business_rules::payment::voucher_1001_fixture;

    #[test]
    fn sha256_abc_is_stable() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn same_input_same_hash() {
        let v = voucher_1001_fixture();
        let a = voucher_fingerprint(
            v.voucher_number,
            "",
            &v.tax_invoice,
            &v.vendor,
            &v.bank,
            &v.account_number,
            &v.ifsc,
            &v.gst,
            &v.project,
            &v.payments,
        );
        let b = voucher_fingerprint(
            v.voucher_number,
            "",
            &v.tax_invoice,
            &v.vendor,
            &v.bank,
            &v.account_number,
            &v.ifsc,
            &v.gst,
            &v.project,
            &v.payments,
        );
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn trim_and_blank_numeric_do_not_change_hash() {
        let v = voucher_1001_fixture();
        let base = voucher_fingerprint(
            20,
            " 2026-01-10 ",
            "INV",
            "VEND",
            "",
            "",
            "",
            "",
            "CUST",
            &v.payments,
        );
        let again = voucher_fingerprint(
            20,
            "2026-01-10",
            "INV",
            "VEND",
            "",
            "",
            "",
            "",
            "CUST",
            &v.payments,
        );
        assert_eq!(base, again);
        assert_eq!(canon_num(None), "0");
        assert_eq!(canon_num(Some(0.0)), "0");
        assert_eq!(canon_num(Some(10.5)), "10.5");
    }

    #[test]
    fn field_change_changes_hash() {
        let v = voucher_1001_fixture();
        let a = voucher_fingerprint(
            20,
            "d",
            "INV",
            "VEND",
            "",
            "",
            "",
            "",
            "CUST",
            &v.payments,
        );
        let b = voucher_fingerprint(
            20,
            "d",
            "INV-2",
            "VEND",
            "",
            "",
            "",
            "",
            "CUST",
            &v.payments,
        );
        assert_ne!(a, b);
    }
}
