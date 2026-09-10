//! E. Two PCs, same voucher. Dirty guard, no silent overwrite.

use t_books_lib::testdata::{parse_clean_one_block, temp_db, voucher_row};
use t_books_lib::{
    apply_voucher_rows, apply_voucher_rows_discarding_dirty, dirty_guard, get_voucher, mark_dirty,
    open_at, parse_voucher_values,
};

#[test]
fn pc_b_refresh_does_not_clobber_pc_a_dirty_row() {
    let path = temp_db("multi");
    {
        let mut pc_a = open_at(&path).unwrap();
        apply_voucher_rows(&mut pc_a, &parse_voucher_values(&[voucher_row(1001, 0.4)])).unwrap();
        mark_dirty(&pc_a, 1001).unwrap();
        pc_a.conn()
            .execute(
                "UPDATE vouchers SET vendor = 'LOCAL_A' WHERE voucher_number = 1001",
                [],
            )
            .unwrap();
    }
    let mut pc_b = open_at(&path).unwrap();
    let dirty = dirty_guard(&pc_b).unwrap().expect("blocked");
    assert_eq!(dirty, vec![1001]);
    let mut incoming = voucher_row(1001, 0.9);
    incoming[3] = "SHEET_B".into();
    apply_voucher_rows(&mut pc_b, &parse_voucher_values(&[incoming.clone()])).unwrap();
    let kept = get_voucher(&pc_b, 1001).unwrap();
    assert_eq!(kept.vendor, "LOCAL_A");
    apply_voucher_rows_discarding_dirty(&mut pc_b, &parse_voucher_values(&[incoming])).unwrap();
    let after = get_voucher(&pc_b, 1001).unwrap();
    assert_eq!(after.vendor, "SHEET_B");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn two_connections_wal_no_panic() {
    let path = temp_db("wal");
    {
        let mut seed = open_at(&path).unwrap();
        apply_voucher_rows(&mut seed, &parse_clean_one_block(40)).unwrap();
    }
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let a = open_at(&path).unwrap();
            for n in 1..=20 {
                mark_dirty(&a, n).unwrap();
            }
        });
        scope.spawn(|| {
            let b = open_at(&path).unwrap();
            let _ = dirty_guard(&b).unwrap();
            let _ = get_voucher(&b, 1).unwrap();
        });
    });
    let check = open_at(&path).unwrap();
    assert!(dirty_guard(&check).unwrap().is_some());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
