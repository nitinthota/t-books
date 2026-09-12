//! Indian financial year: 1 Apr–31 Mar. Dummy dates only in tests.

use std::time::{SystemTime, UNIX_EPOCH};

fn parse_iso_ymd(iso: &str) -> Option<(i32, u32, u32)> {
    let d = iso.trim();
    let d = if d.len() >= 10 { &d[..10] } else { d };
    if d.len() != 10 || d.as_bytes().get(4) != Some(&b'-') || d.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    let y: i32 = d[0..4].parse().ok()?;
    let m: u32 = d[5..7].parse().ok()?;
    let day: u32 = d[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    Some((y, m, day))
}

/// Days since Unix epoch → civil UTC date (Howard Hinnant).
fn civil_from_unix_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub fn utc_today() -> (i32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_from_unix_days((secs / 86_400) as i64)
}

fn ymd_from(today_iso: Option<&str>) -> (i32, u32, u32) {
    today_iso.and_then(parse_iso_ymd).unwrap_or_else(utc_today)
}

fn fy_label(start_year: i32) -> String {
    format!("{}-{:02}", start_year, ((start_year + 1) as i32).rem_euclid(100))
}

fn start_year_for(y: i32, month_1: u32) -> i32 {
    if month_1 >= 4 {
        y
    } else {
        y - 1
    }
}

pub fn current_indian_fy(today_iso: Option<&str>) -> String {
    let (y, m, _) = ymd_from(today_iso);
    fy_label(start_year_for(y, m))
}

pub fn fy_bounds(fy: &str) -> (String, String) {
    let start_year: i32 = fy.trim().get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    if !(1990..=2100).contains(&start_year) {
        return fy_bounds(&current_indian_fy(None));
    }
    let end_year = start_year + 1;
    (
        format!("{start_year}-04-01"),
        format!("{end_year}-03-31"),
    )
}

pub fn fy_options(min_date: Option<&str>, max_date: Option<&str>, today_iso: Option<&str>) -> Vec<String> {
    let mut years = std::collections::BTreeSet::new();
    let mut push = |iso: Option<&str>| {
        if let Some((y, m, _)) = iso.and_then(parse_iso_ymd) {
            years.insert(start_year_for(y, m));
        }
    };
    push(min_date);
    push(max_date);
    let cur = current_indian_fy(today_iso);
    if let Ok(y) = cur[..4].parse::<i32>() {
        years.insert(y);
    }
    let mut out: Vec<String> = years.into_iter().rev().map(fy_label).collect();
    out.sort_by(|a, b| b.cmp(a));
    out
}

pub fn clamp_as_of(as_of: Option<&str>, start: &str, end: &str, today_iso: &str) -> String {
    let raw = as_of.unwrap_or(today_iso);
    let raw = if raw.len() >= 10 { &raw[..10] } else { raw };
    if raw < start {
        start.to_string()
    } else if raw > end {
        end.to_string()
    } else {
        raw.to_string()
    }
}

pub fn trial_closing(opening_paise: i64, period_dr: i64, period_cr: i64) -> i64 {
    opening_paise + period_dr - period_cr
}

pub fn signed_dr_cr(paise: i64) -> (i64, i64) {
    if paise > 0 {
        (paise, 0)
    } else if paise < 0 {
        (0, -paise)
    } else {
        (0, 0)
    }
}

pub fn trial_balanced(rows: &[(i64, i64)]) -> bool {
    let dr: i64 = rows.iter().map(|r| r.0).sum();
    let cr: i64 = rows.iter().map(|r| r.1).sum();
    dr == cr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indian_fy_starts_1_apr() {
        assert_eq!(current_indian_fy(Some("2026-04-01")), "2026-27");
        assert_eq!(current_indian_fy(Some("2026-03-31")), "2025-26");
        assert_eq!(current_indian_fy(Some("2026-09-12")), "2026-27");
    }

    #[test]
    fn fy_bounds_are_1_apr_31_mar() {
        let (start, end) = fy_bounds("2026-27");
        assert_eq!(start, "2026-04-01");
        assert_eq!(end, "2027-03-31");
    }

    #[test]
    fn trial_must_balance() {
        assert_eq!(trial_closing(100, 40, 25), 115);
        assert_eq!(signed_dr_cr(50), (50, 0));
        assert_eq!(signed_dr_cr(-50), (0, 50));
        assert!(trial_balanced(&[(100, 0), (0, 100)]));
        assert!(!trial_balanced(&[(100, 0), (0, 90)]));
    }

    #[test]
    fn clamp_as_of_stays_in_fy() {
        assert_eq!(
            clamp_as_of(Some("2025-01-01"), "2026-04-01", "2027-03-31", "2026-09-12"),
            "2026-04-01"
        );
        assert_eq!(
            clamp_as_of(Some("2028-01-01"), "2026-04-01", "2027-03-31", "2026-09-12"),
            "2027-03-31"
        );
    }
}
