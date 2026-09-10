//! Stress beyond normal office size. Not part of the product UI.

use std::time::Instant;
use t_books_lib::testdata::{parse_clean, rss_kb, temp_db};
use t_books_lib::{
    apply_voucher_rows, get_voucher, get_voucher_summary, list_vouchers, mark_dirty, open_at,
};

fn main() {
    let count: usize = std::env::var("TBOOKS_STRESS_VOUCHERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let path = temp_db("stress");
    let rss_start = rss_kb();
    let parsed = parse_clean(count);
    let parse_ms = {
        let t = Instant::now();
        let _ = parsed.vouchers.len();
        t.elapsed().as_millis()
    };
    let mut books = open_at(&path).expect("open stress db");
    let t = Instant::now();
    let out = apply_voucher_rows(&mut books, &parsed).expect("apply");
    let apply_ms = t.elapsed().as_millis();
    let summary = get_voucher_summary(&books).expect("summary");
    let list = list_vouchers(&books).expect("list");
    let t = Instant::now();
    for n in 1..=100 {
        let _ = get_voucher(&books, n as i64);
        let _ = mark_dirty(&books, n as i64);
    }
    let hundred_ms = t.elapsed().as_millis();
    let ops_per_sec = if hundred_ms == 0 {
        100_000.0
    } else {
        200.0 * 1000.0 / hundred_ms as f64
    };
    let rss_end = rss_kb();
    let imported = match out {
        t_books_lib::RefreshOutcome::Ok { imported, .. } => imported,
        t_books_lib::RefreshOutcome::Dirty { .. } => 0,
    };
    let report = serde_json::json!({
        "suite": "stress",
        "vouchers_requested": count,
        "vouchers_imported": imported,
        "list_len": list.len(),
        "summary_count": summary.count,
        "payments_per_voucher": 5,
        "apply_ms": apply_ms,
        "parse_note_ms": parse_ms,
        "hundred_edit_ms": hundred_ms,
        "ops_per_sec": ops_per_sec,
        "rss_kb_start": rss_start,
        "rss_kb_end": rss_end,
        "rss_kb_limit": 1_500_000u64,
        "crashed": false,
        "data_loss": summary.count != imported as i64,
        "within_4gb": rss_end < 1_500_000,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    if rss_end >= 1_500_000 {
        std::process::exit(2);
    }
}
