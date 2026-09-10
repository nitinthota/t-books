//! F/G. Failures and recovery: Google, corrupt file, rollback, keep data.

use t_books_lib::testdata::{parse_clean_one_block, temp_db};
use t_books_lib::{
    apply_voucher_rows, force_refresh_vouchers, list_vouchers, open_at, open_memory, refresh_vouchers,
};

#[test]
fn google_or_credentials_failure_keeps_rows() {
    let mut books = open_memory().unwrap();
    apply_voucher_rows(&mut books, &parse_clean_one_block(3)).unwrap();
    assert!(refresh_vouchers(&mut books).is_err());
    assert!(force_refresh_vouchers(&mut books).is_err());
    assert_eq!(list_vouchers(&books).unwrap().len(), 3);
}

#[test]
fn uncommitted_transaction_rolls_back() {
    let path = temp_db("tx");
    let mut books = open_at(&path).unwrap();
    {
        let tx = books.conn_mut().transaction().unwrap();
        tx.execute(
            "INSERT INTO vendors (vendor, updated_at) VALUES ('TEMP', datetime('now'))",
            [],
        )
        .unwrap();
        drop(tx);
    }
    let n: i64 = books
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM vendors WHERE vendor = 'TEMP'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn corrupt_file_is_a_real_error_not_a_panic() {
    let path = temp_db("corrupt");
    std::fs::write(&path, b"this is not a sqlite database").unwrap();
    match open_at(&path) {
        Ok(_) => panic!("corrupt file opened"),
        Err(err) => assert!(!err.to_string().is_empty()),
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn reopen_after_apply_sees_committed_rows() {
    let path = temp_db("reopen");
    {
        let mut books = open_at(&path).unwrap();
        apply_voucher_rows(&mut books, &parse_clean_one_block(5)).unwrap();
    }
    let books = open_at(&path).unwrap();
    assert_eq!(list_vouchers(&books).unwrap().len(), 5);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn directory_as_db_path_is_a_real_error() {
    let path = temp_db("isdir");
    let dir = path.parent().unwrap().join("not-a-file");
    std::fs::create_dir_all(&dir).unwrap();
    match open_at(&dir) {
        Ok(_) => panic!("opened a directory as the books file"),
        Err(err) => assert!(!err.to_string().is_empty()),
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
