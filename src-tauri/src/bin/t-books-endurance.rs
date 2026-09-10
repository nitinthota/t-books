//! Long-run soak. Default is a short sample; set TBOOKS_ENDURANCE_SECS for hours.

use std::time::{Duration, Instant};
use t_books_lib::testdata::{parse_clean_one_block, rss_kb, temp_db};
use t_books_lib::{apply_voucher_rows, get_voucher, get_voucher_summary, list_vouchers, open_at};

fn main() {
    let secs: u64 = std::env::var("TBOOKS_ENDURANCE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let path = temp_db("endurance");
    let mut books = open_at(&path).expect("open");
    apply_voucher_rows(&mut books, &parse_clean_one_block(400)).expect("seed");
    let rss0 = rss_kb();
    let start = Instant::now();
    let mut loops = 0u64;
    let mut slow = 0u64;
    while start.elapsed() < Duration::from_secs(secs) {
        let t = Instant::now();
        let _ = get_voucher_summary(&books).unwrap();
        let _ = list_vouchers(&books).unwrap();
        let _ = get_voucher(&books, ((loops % 400) + 1) as i64).unwrap();
        if t.elapsed().as_millis() > 250 {
            slow += 1;
        }
        loops += 1;
    }
    let rss1 = rss_kb();
    let growth = rss1.saturating_sub(rss0);
    let report = serde_json::json!({
        "suite": "endurance",
        "seconds": secs,
        "loops": loops,
        "slow_ops": slow,
        "rss_kb_start": rss0,
        "rss_kb_end": rss1,
        "rss_kb_growth": growth,
        "leak_suspected": growth > 80_000,
        "crashed": false,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    if growth > 80_000 {
        std::process::exit(2);
    }
}
