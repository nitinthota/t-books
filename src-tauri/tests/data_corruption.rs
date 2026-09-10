//! B. Bad data: skip invalid rows, never crash, keep local books.

use t_books_lib::testdata::{mixed_sheet, parse_mixed, voucher_row};
use t_books_lib::{apply_voucher_rows, list_vouchers, open_memory, parse_voucher_values};

#[test]
fn decimal_string_negative_voucher_numbers_are_skipped() {
    let parsed = parse_voucher_values(&[
        voucher_row(20, 0.4),
        {
            let mut r = voucher_row(21, 0.4);
            r[0] = "20.1".into();
            r
        },
        {
            let mut r = voucher_row(22, 0.4);
            r[0] = "ABC".into();
            r
        },
        {
            let mut r = voucher_row(23, 0.4);
            r[0] = "-9".into();
            r
        },
    ]);
    assert_eq!(parsed.vouchers.len(), 1);
    assert_eq!(parsed.vouchers[0].voucher_number, 20);
    assert_eq!(parsed.skipped, 3);
    assert!(parsed
        .errors
        .iter()
        .all(|e| e.error_type == "invalid_voucher_number"));
}

#[test]
fn null_text_huge_and_negative_money_do_not_panic() {
    let mut row = voucher_row(30, 0.4);
    row[9 + 2] = "not-a-number".into();
    row[9 + 5] = String::new();
    row[9 + 6] = "-12".into();
    let mut huge = voucher_row(31, 0.4);
    huge[9 + 2] = "1000000000000".into();
    let parsed = parse_voucher_values(&[row, huge]);
    assert_eq!(parsed.vouchers.len(), 2);
    assert_eq!(parsed.vouchers[0].payments[0].pi_value_rupees, Some(0.0));
    assert!(parsed.vouchers[1].payments[0].pi_value_rupees.unwrap() > 0.0);
}

#[test]
fn invalid_and_empty_dates_are_kept_as_text() {
    let mut row = voucher_row(40, 0.4);
    row[1] = "not-a-date".into();
    row[9 + 1] = String::new();
    let parsed = parse_voucher_values(&[row]);
    assert_eq!(parsed.vouchers.len(), 1);
    assert_eq!(parsed.vouchers[0].voucher_date, "not-a-date");
}

#[test]
fn duplicate_voucher_numbers_last_valid_wins_without_crash() {
    let a = voucher_row(50, 0.2);
    let mut b = voucher_row(50, 0.8);
    b[3] = "VEND_DUP".into();
    let parsed = parse_voucher_values(&[a, b]);
    assert_eq!(parsed.vouchers.len(), 1);
    assert_eq!(parsed.vouchers[0].vendor, "VEND_DUP");
}

#[test]
fn mixed_sheet_apply_never_wipes_valid_rows() {
    let mut books = open_memory().unwrap();
    let parsed = parse_mixed(80);
    assert!(parsed.skipped > 0);
    apply_voucher_rows(&mut books, &parsed).unwrap();
    let list = list_vouchers(&books).unwrap();
    assert_eq!(list.len(), parsed.vouchers.len());
    assert!(!list.is_empty());
}

#[test]
fn mixed_sheet_builder_includes_the_required_edge_cases() {
    let sheet = mixed_sheet(46);
    let col_a: Vec<&str> = sheet.iter().skip(1).map(|r| r[0].as_str()).collect();
    assert!(col_a.iter().any(|a| *a == "20.1"));
    assert!(col_a.iter().any(|a| *a == "ABC"));
    assert!(col_a.iter().any(|a| *a == "-9"));
}
