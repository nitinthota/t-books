//! C. Stress (cargo): 2k vouchers × 5 payments, rapid edits, RSS bound.

use std::time::Instant;
use t_books_lib::testdata::{parse_clean, rss_kb};
use t_books_lib::{
    apply_voucher_rows, get_voucher, get_voucher_summary, list_vouchers, mark_dirty, open_memory,
};

#[test]
fn two_thousand_vouchers_five_blocks_no_crash() {
    let rss0 = rss_kb();
    let parsed = parse_clean(2_000);
    assert_eq!(parsed.vouchers.len(), 2_000);
    let mut books = open_memory().unwrap();
    let t = Instant::now();
    apply_voucher_rows(&mut books, &parsed).unwrap();
    let apply_ms = t.elapsed().as_millis();
    let summary = get_voucher_summary(&books).unwrap();
    assert_eq!(summary.count, 2000);
    let list = list_vouchers(&books).unwrap();
    assert_eq!(list.len(), 2000);
    let t = Instant::now();
    for n in 1..=80 {
        get_voucher(&books, n).unwrap();
        mark_dirty(&books, n).unwrap();
    }
    let edit_ms = t.elapsed().as_millis();
    let rss1 = rss_kb();
    assert!(
        rss1 < 1_500_000,
        "RSS {rss1} KB exceeds 1.5 GB bound (start {rss0})"
    );
    assert!(apply_ms < 30_000, "apply took {apply_ms} ms");
    assert!(edit_ms < 5_000, "edits took {edit_ms} ms");
}
