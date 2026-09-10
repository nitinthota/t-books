//! J. Write-back fingerprint tests (Step 7 suite).

use t_books_lib::testdata::voucher_row;
use t_books_lib::{
    apply_voucher_rows, conflict_message, mark_dirty, open_memory, parse_voucher_values,
    submit_voucher_with, MemorySheet, SubmitOutcome,
};

fn seed(n: i64) -> (t_books_lib::LocalBooks, MemorySheet) {
    let mut books = open_memory().unwrap();
    let row = voucher_row(n, 0.4);
    apply_voucher_rows(&mut books, &parse_voucher_values(&[row.clone()])).unwrap();
    let mut sheet = MemorySheet::with_header();
    sheet.rows.push(row);
    (books, sheet)
}

#[test]
fn same_data_write_succeeds() {
    let (mut books, mut sheet) = seed(1001);
    assert_eq!(
        submit_voucher_with(&mut books, 1001, &mut sheet).unwrap(),
        SubmitOutcome::Ok {
            voucher_number: 1001
        }
    );
}

#[test]
fn modified_remote_conflicts() {
    let (mut books, mut sheet) = seed(1001);
    sheet.rows[1][3] = "OTHER".into();
    match submit_voucher_with(&mut books, 1001, &mut sheet).unwrap() {
        SubmitOutcome::Conflict { message, .. } => {
            assert_eq!(message, conflict_message(1001));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn multi_pc_second_submit_conflicts() {
    let (mut a, mut sheet) = seed(1001);
    let mut b = open_memory().unwrap();
    apply_voucher_rows(&mut b, &parse_voucher_values(&[voucher_row(1001, 0.4)])).unwrap();
    a.conn()
        .execute(
            "UPDATE vouchers SET vendor = 'PC_A' WHERE voucher_number = 1001",
            [],
        )
        .unwrap();
    submit_voucher_with(&mut a, 1001, &mut sheet).unwrap();
    b.conn()
        .execute(
            "UPDATE vouchers SET vendor = 'PC_B' WHERE voucher_number = 1001",
            [],
        )
        .unwrap();
    mark_dirty(&b, 1001).unwrap();
    match submit_voucher_with(&mut b, 1001, &mut sheet).unwrap() {
        SubmitOutcome::Conflict { voucher_number, .. } => assert_eq!(voucher_number, 1001),
        other => panic!("{other:?}"),
    }
}

#[test]
fn network_failure_keeps_local() {
    let (mut books, mut sheet) = seed(1001);
    mark_dirty(&books, 1001).unwrap();
    sheet.fail_writes = true;
    assert!(submit_voucher_with(&mut books, 1001, &mut sheet).is_err());
    let dirty: i64 = books
        .conn()
        .query_row(
            "SELECT is_dirty FROM vouchers WHERE voucher_number = 1001",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dirty, 1);
}

#[test]
fn invalid_data_rejected() {
    let mut books = open_memory().unwrap();
    let mut sheet = MemorySheet::with_header();
    assert!(submit_voucher_with(&mut books, 0, &mut sheet).is_err());
    assert!(submit_voucher_with(&mut books, 404, &mut sheet).is_err());
}
