//! E. Two PCs, same voucher. Dirty guard, no silent overwrite.
//! Agents 8–9: 3-PC fake hive, PUR CAS, dummy skip, no LWW.

use t_books_lib::{
    apply_voucher_rows, apply_voucher_rows_discarding_dirty, dirty_guard, get_voucher,
    hive_conflict_message, mark_dirty, open_at, open_memory, parse_voucher_values,
    save_purchase_payment, save_purchase_po, submit_office_with, tab_headers, CasOutcome, Hive,
    MemoryHive, PurchasePaymentSave, PurchasePoSave, KIND_PAYMENT, KIND_PURCHASE,
};
use t_books_lib::testdata::{parse_clean_one_block, temp_db, voucher_row};

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

fn simple_bill(number: &str, total: f64) -> PurchasePoSave {
    PurchasePoSave {
        project: "CUST_01".into(),
        vendor: "VEND_02".into(),
        po_number: number.into(),
        po_type: "simple".into(),
        total_value: total,
        ..Default::default()
    }
}

#[test]
fn three_pcs_same_pur_second_and_third_conflict() {
    let mut hive = MemoryHive::default();
    hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE)).unwrap();
    let mut pc_a = open_memory().unwrap();
    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 10.0)).unwrap();
    let first = submit_office_with(&pc_a, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap();
    assert!(matches!(first, CasOutcome::Ok { .. }));

    let mut pc_b = open_memory().unwrap();
    save_purchase_po(&mut pc_b, simple_bill("PUR-0001", 20.0)).unwrap();
    match submit_office_with(&pc_b, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap() {
        CasOutcome::Conflict { key, message } => {
            assert_eq!(key, "PUR-0001");
            assert_eq!(message, hive_conflict_message("PUR-0001"));
        }
        other => panic!("{other:?}"),
    }

    let mut pc_c = open_memory().unwrap();
    save_purchase_po(&mut pc_c, simple_bill("PUR-0001", 30.0)).unwrap();
    match submit_office_with(&pc_c, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap() {
        CasOutcome::Conflict { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn different_keys_do_not_block() {
    let mut hive = MemoryHive::default();
    hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE)).unwrap();
    hive.ensure_tab(KIND_PAYMENT, &tab_headers(KIND_PAYMENT)).unwrap();
    let mut pc_a = open_memory().unwrap();
    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 10.0)).unwrap();
    let mut pc_b = open_memory().unwrap();
    save_purchase_po(&mut pc_b, simple_bill("PUR-0002", 40.0)).unwrap();
    save_purchase_payment(
        &mut pc_b,
        PurchasePaymentSave {
            po_number: "PUR-0002".into(),
            amount_rupees: 10.0,
            ..Default::default()
        },
    )
    .unwrap();
    let a = submit_office_with(&pc_a, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap();
    let b = submit_office_with(&pc_b, &mut hive, KIND_PURCHASE, "PUR-0002", true).unwrap();
    let p = submit_office_with(&pc_b, &mut hive, KIND_PAYMENT, "PAY-0001", true).unwrap();
    assert!(matches!(a, CasOutcome::Ok { .. }));
    assert!(matches!(b, CasOutcome::Ok { .. }));
    assert!(matches!(p, CasOutcome::Ok { .. }));
}

#[test]
fn dummy_serials_never_allocate_and_dotted_child_is_not_a_row() {
    use t_books_lib::business_rules::{is_dotted_child_serial, is_dummy_serial, next_purchase_number};
    assert!(is_dummy_serial("VOUCHER_1001"));
    assert!(is_dummy_serial("CUST_01"));
    assert!(is_dummy_serial("VEND_02"));
    assert!(is_dotted_child_serial("20.1"));
    assert_eq!(
        next_purchase_number(&["VOUCHER_1001".into(), "SAMPLE".into(), "PUR-0001".into()]),
        "PUR-0002"
    );
}

#[test]
fn no_lww_on_money_conflict_keeps_first_hive_total() {
    let mut hive = MemoryHive::default();
    hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE)).unwrap();
    let mut pc_a = open_memory().unwrap();
    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 10.0)).unwrap();
    submit_office_with(&pc_a, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap();
    let mut pc_b = open_memory().unwrap();
    save_purchase_po(&mut pc_b, simple_bill("PUR-0001", 99.0)).unwrap();
    let conflict = submit_office_with(&pc_b, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap();
    assert!(matches!(conflict, CasOutcome::Conflict { .. }));
    let remote = hive.get_row(KIND_PURCHASE, "PUR-0001").unwrap().unwrap();
    assert!(remote.cells.iter().any(|c| c == "10" || c == "10.0"));
}
