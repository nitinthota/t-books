//! Synthetic voucher sheets for validation. Not used by the UI.

use crate::vouchers::parse_voucher_values;
use crate::vouchers::ParseReport;

pub fn voucher_row(number: i64, paid_frac: f64) -> Vec<String> {
    let value = 10_000.0;
    let paid = (value * paid_frac).round();
    let remaining = value - paid;
    let mut row = vec![
        number.to_string(),
        "2026-01-10".into(),
        format!("INV-{number}"),
        format!("VEND_{:02}", (number % 17) + 1),
        "".into(),
        "".into(),
        "".into(),
        "".into(),
        format!("CUST_{:02}", (number % 9) + 1),
    ];
    for slot in 1..=5 {
        if slot == 1 {
            row.extend([
                format!("PI-{number}"),
                "2026-01-11".into(),
                value.to_string(),
                "40".into(),
                "Block".into(),
                "0".into(),
                paid.to_string(),
                "UTR".into(),
                "2026-01-12".into(),
                remaining.to_string(),
                "".into(),
            ]);
        } else {
            row.extend(std::iter::repeat(String::new()).take(11));
        }
    }
    row
}

pub fn voucher_row_full_five(number: i64) -> Vec<String> {
    let mut row = vec![
        number.to_string(),
        "2026-01-10".into(),
        format!("INV-{number}"),
        "VEND_01".into(),
        "".into(),
        "".into(),
        "".into(),
        "".into(),
        "CUST_01".into(),
    ];
    for slot in 1..=5 {
        row.extend([
            format!("PI-{number}-{slot}"),
            "2026-01-11".into(),
            "2000".into(),
            "20".into(),
            "Block".into(),
            "0".into(),
            "400".into(),
            "UTR".into(),
            "2026-01-12".into(),
            "1600".into(),
            "".into(),
        ]);
    }
    row
}

/// Mix real rows with the bad cells Refresh must skip, never crash on.
pub fn mixed_sheet(valid: usize) -> Vec<Vec<String>> {
    let mut rows = vec![vec![
        "voucher_number".into(),
        "date".into(),
        "tax inv".into(),
        "vendor".into(),
    ]];
    for i in 1..=valid {
        match i % 23 {
            0 => {
                let mut bad = voucher_row(i as i64, 0.4);
                bad[0] = "20.1".into();
                rows.push(bad);
            }
            1 if i > 1 => {
                let mut bad = voucher_row(i as i64, 0.4);
                bad[0] = "ABC".into();
                rows.push(bad);
            }
            2 if i > 2 => {
                let mut bad = voucher_row(i as i64, 0.4);
                bad[0] = "-9".into();
                rows.push(bad);
            }
            3 => {
                let mut row = voucher_row(i as i64, 0.4);
                row[9 + 2] = "not-a-number".into();
                row[9 + 6] = "".into();
                rows.push(row);
            }
            4 => {
                let mut row = voucher_row(i as i64, 0.4);
                row[9 + 2] = "1000000000000".into();
                rows.push(row);
            }
            5 => {
                let mut row = voucher_row(i as i64, 0.4);
                row[9 + 6] = "-50".into();
                rows.push(row);
            }
            6 => {
                let mut row = voucher_row(i as i64, 0.4);
                row[1] = "not-a-date".into();
                rows.push(row);
            }
            _ => rows.push(voucher_row(i as i64, if i % 3 == 0 { 1.0 } else { 0.4 })),
        }
    }
    rows
}

pub fn parse_mixed(valid: usize) -> ParseReport {
    parse_voucher_values(&mixed_sheet(valid))
}

pub fn parse_clean(count: usize) -> ParseReport {
    let mut rows = vec![vec!["voucher_number".into(), "date".into(), "tax inv".into()]];
    for i in 1..=count {
        rows.push(voucher_row_full_five(i as i64));
    }
    parse_voucher_values(&rows)
}

pub fn parse_clean_one_block(count: usize) -> ParseReport {
    let mut rows = vec![vec!["voucher_number".into(), "date".into(), "tax inv".into()]];
    for i in 1..=count {
        rows.push(voucher_row(i as i64, 0.4));
    }
    parse_voucher_values(&rows)
}

pub fn rss_kb() -> u64 {
    let Ok(text) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

pub fn temp_db(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tbooks-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("tbooks.db")
}
