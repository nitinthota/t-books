//! Hive-side number allocation. Dummy rows never allocate. Posted numbers stay put.

use super::hr_payroll::keep_posted_pay_number;
use super::payment::is_dummy_serial;
use super::purchase_status::{format_payment_number, parse_pay_voucher_seq};

fn digits_after_prefix(raw: &str, prefix: &str) -> i64 {
    let s = raw.trim();
    let p = prefix.trim();
    if p.is_empty() {
        return 0;
    }
    if s.len() < p.len() {
        return 0;
    }
    if !s.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p)) {
        return 0;
    }
    let rest = &s[p.len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        0
    } else {
        digits.parse().unwrap_or(0)
    }
}

fn live_keys<'a>(existing: &'a [String]) -> impl Iterator<Item = &'a str> {
    existing
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !is_dummy_serial(s) && !s.trim().is_empty())
}

/// Shared PUR-NNNN sequence. Also counts legacy PURC:NNNN.
pub fn next_purchase_number(existing: &[String]) -> String {
    let mut max = 0i64;
    for raw in live_keys(existing) {
        let s = raw.trim();
        let n = if let n @ 1.. = digits_after_prefix(s, "PUR-") {
            n
        } else {
            digits_after_prefix(s, "PURC:")
        };
        if n > max {
            max = n;
        }
    }
    format!("PUR-{:04}", max + 1)
}

pub fn next_converted_po_number(existing: &[String]) -> String {
    next_purchase_number(existing)
}

pub fn next_project_expense_number(existing: &[String]) -> String {
    next_purchase_number(existing)
}

/// Workspace-wide PAY-0001 series. `po_number` is ignored (old call sites).
pub fn next_payment_number(existing: &[String], _po_number: Option<&str>) -> String {
    let mut max = 0i64;
    for raw in live_keys(existing) {
        max = max.max(parse_pay_voucher_seq(Some(raw)));
    }
    format_payment_number(max + 1)
}

pub fn next_salary_number(existing: &[String]) -> String {
    let mut max = 0i64;
    for raw in live_keys(existing) {
        let n = digits_after_prefix(raw.trim(), "SAL-");
        if n > max {
            max = n;
        }
    }
    format!("SAL-{:04}", max + 1)
}

/// Next PREFIX-#### from existing numbers. Reversed suffixes (PREFIX-0004~id) still count.
pub fn allocate_voucher_serial(existing_nos: &[String], prefix: &str) -> String {
    let p = prefix.trim().to_ascii_uppercase();
    let p = if p.is_empty() { "VCH".to_string() } else { p };
    let needle = format!("{p}-");
    let mut max = 0i64;
    for raw in live_keys(existing_nos) {
        let n = digits_after_prefix(raw.trim(), &needle);
        if n > max {
            max = n;
        }
    }
    format!("{p}-{:04}", max + 1)
}

pub fn keep_posted_number(posted_no: &str, requested_no: &str) -> String {
    keep_posted_pay_number(posted_no, requested_no)
}

/// Union of local numbers and hive keys. Dummy rows never count.
pub fn merge_key_pool(local: &[String], hive: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for raw in local.iter().chain(hive.iter()) {
        let t = raw.trim();
        if t.is_empty() || is_dummy_serial(t) {
            continue;
        }
        if seen.insert(t.to_ascii_uppercase()) {
            out.push(t.to_string());
        }
    }
    out
}

pub fn next_purchase_from_hive_and_local(local: &[String], hive: &[String]) -> String {
    next_purchase_number(&merge_key_pool(local, hive))
}

pub fn next_payment_from_hive_and_local(local: &[String], hive: &[String]) -> String {
    next_payment_number(&merge_key_pool(local, hive), None)
}

pub fn next_salary_from_hive_and_local(local: &[String], hive: &[String]) -> String {
    next_salary_number(&merge_key_pool(local, hive))
}

/// Empty requested → allocate from hive+local. Taken number → Err so Submit can bump.
pub fn allocate_or_reject(
    posted: &str,
    requested: &str,
    local: &[String],
    hive: &[String],
    kind: &str,
) -> std::result::Result<String, String> {
    let posted = posted.trim();
    if !posted.is_empty() {
        return Ok(keep_posted_number(posted, requested));
    }
    let want = requested.trim().to_string();
    let pool = merge_key_pool(local, hive);
    if want.is_empty() {
        let next = match kind {
            "purchase" => next_purchase_number(&pool),
            "salary" => next_salary_number(&pool),
            _ => next_payment_number(&pool, None),
        };
        return Ok(next);
    }
    if pool.iter().any(|k| k.eq_ignore_ascii_case(&want)) {
        return Err(format!("{want} is already used. Reload and submit again."));
    }
    Ok(want)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purchase_seq_skips_dummy_and_counts_legacy() {
        let existing = vec![
            "VOUCHER_1001".into(),
            "PUR-0001".into(),
            "PURC:0003".into(),
            "SAMPLE-ROW".into(),
        ];
        assert_eq!(next_purchase_number(&existing), "PUR-0004");
        assert_eq!(next_purchase_number(&[]), "PUR-0001");
    }

    #[test]
    fn payment_and_salary_allocate_from_hive_list() {
        let pays = vec!["PAY-0001".into(), "PAY:2".into(), "CUST_01".into()];
        assert_eq!(next_payment_number(&pays, Some("PUR-0001")), "PAY-0003");
        let sals = vec!["SAL-0001".into(), "SAL-0001~rev".into()];
        assert_eq!(next_salary_number(&sals), "SAL-0002");
    }

    #[test]
    fn allocate_voucher_serial_counts_reversed_suffix() {
        let nos = vec!["VCH-0004~id".into(), "VCH-0002".into(), "VOUCHER_1001".into()];
        assert_eq!(allocate_voucher_serial(&nos, "VCH"), "VCH-0005");
        assert_eq!(allocate_voucher_serial(&[], ""), "VCH-0001");
    }

    #[test]
    fn posted_number_is_not_stolen() {
        assert_eq!(keep_posted_number("PAY-0001", "PAY-0099"), "PAY-0001");
    }

    #[test]
    fn hive_plus_local_bumps_taken_pay() {
        let local = vec!["PAY-0001".into(), "VOUCHER_1001".into()];
        let hive = vec!["PAY-0002".into(), "CUST_01".into()];
        assert_eq!(next_payment_from_hive_and_local(&local, &hive), "PAY-0003");
        let err = allocate_or_reject("", "PAY-0002", &local, &hive, "payment").unwrap_err();
        assert!(err.contains("already used"));
        assert_eq!(
            allocate_or_reject("PAY-0001", "PAY-0099", &local, &hive, "payment").unwrap(),
            "PAY-0001"
        );
        assert_eq!(
            next_purchase_from_hive_and_local(&["PUR-0001".into()], &["PURC:0004".into()]),
            "PUR-0005"
        );
    }
}
