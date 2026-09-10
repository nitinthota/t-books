//! H. Endurance sample. Full 24h is TBOOKS_ENDURANCE_SECS on the bin.

use std::time::Instant;
use t_books_lib::testdata::{parse_clean_one_block, rss_kb};
use t_books_lib::{apply_voucher_rows, get_voucher, get_voucher_summary, list_vouchers, open_memory};

#[test]
fn four_hundred_read_loops_do_not_leak_unbounded() {
    let mut books = open_memory().unwrap();
    apply_voucher_rows(&mut books, &parse_clean_one_block(120)).unwrap();
    let rss0 = rss_kb();
    let start = Instant::now();
    for i in 0..400 {
        let _ = get_voucher_summary(&books).unwrap();
        let _ = list_vouchers(&books).unwrap();
        let _ = get_voucher(&books, ((i % 120) + 1) as i64).unwrap();
    }
    let ms = start.elapsed().as_millis();
    let rss1 = rss_kb();
    let growth = rss1.saturating_sub(rss0);
    assert!(ms < 15_000, "endurance sample {ms} ms");
    assert!(growth < 80_000, "RSS grew {growth} KB");
}
