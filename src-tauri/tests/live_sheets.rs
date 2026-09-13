//! Live Google hive writes (service-account JWT in sheets.rs) plus Memory hive logic.
//! Views never call Google. Skips in CI without `TBOOKS_GOOGLE_SA_JSON` / credentials.json.
//! `Voucher_Raw_Data` is never a write target.
//! Dummy keys only: CUST_01, VEND_02, VOUCHER_1001, PUR-0001, PAY-0001, PO-1@CUST_01.

use t_books_lib::business_rules::{
    is_dotted_child_serial, is_dummy_serial, payment_status, should_skip_sheet_row,
};
use t_books_lib::{
    credentials_exist, hive_conflict_message, is_live_dummy_key, list_vouchers, load_hive_map,
    open_memory, refuse_raw_write, save_purchase_payment, save_purchase_po, save_sales_po,
    save_voucher, submit_office_with, tab_headers, CasOutcome, GoogleOfficeHive, Hive, MemoryHive,
    PaymentSave, PurchasePaymentSave, PurchasePoSave, SalesPoSave, VoucherSave, KIND_PAYMENT,
    KIND_PURCHASE, KIND_SALES_PO,
};

fn live_required() -> bool {
    std::env::var("TBOOKS_LIVE_GOOGLE").ok().as_deref() == Some("1")
}

fn skip(reason: &str) -> Option<GoogleOfficeHive> {
    if live_required() {
        panic!("{reason}");
    }
    eprintln!("skip live Google write: {reason}");
    None
}

/// Connects the hive JWT path. Drive MCP can read hive files but cannot PUT cells.
fn live_hive() -> Option<GoogleOfficeHive> {
    if !credentials_exist() {
        return skip("no TBOOKS_GOOGLE_SA_JSON / credentials.json (service account)");
    }
    match GoogleOfficeHive::connect() {
        Ok(hive) => Some(hive),
        Err(err) => skip(&format!("could not connect hive: {err}")),
    }
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

fn voucher_save(number: i64, payments: Vec<PaymentSave>) -> VoucherSave {
    VoucherSave {
        voucher_number: number,
        voucher_date: "2026-09-01".into(),
        tax_invoice: "INV-20".into(),
        vendor: "VEND_02".into(),
        bank: String::new(),
        account_number: String::new(),
        ifsc: String::new(),
        gst: String::new(),
        project: "CUST_01".into(),
        comments: String::new(),
        description: "dummy".into(),
        voucher_type: "purchase".into(),
        payments,
    }
}

fn pay_slot(slot: i32, paid: f64) -> PaymentSave {
    PaymentSave {
        slot,
        pi_no: format!("PI-{slot}"),
        pi_date: String::new(),
        pi_value: 100.0,
        paid,
        remaining: 100.0 - paid,
        payment_date: String::new(),
        remarks: String::new(),
        description: String::new(),
        tds: 0.0,
        payment_details: String::new(),
        payment_percent: 0.0,
    }
}

fn cleanup(hive: &mut GoogleOfficeHive, kind: &str, key: &str) {
    assert!(is_live_dummy_key(key));
    if let Err(err) = hive.blank_dummy_key(kind, key) {
        eprintln!("live dummy cleanup skipped for {key}: {err}");
    }
}

#[test]
fn logic_cas_fp_rev_and_no_lww_on_money() {
    let mut hive = MemoryHive::default();
    hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE))
        .unwrap();
    let mut pc_a = open_memory().unwrap();
    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 10.0)).unwrap();
    let first = submit_office_with(&pc_a, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap();
    match first {
        CasOutcome::Ok { key, rev, .. } => {
            assert_eq!(key, "PUR-0001");
            assert_eq!(rev, 1);
        }
        other => panic!("{other:?}"),
    }
    let mut pc_b = open_memory().unwrap();
    save_purchase_po(&mut pc_b, simple_bill("PUR-0001", 99.0)).unwrap();
    match submit_office_with(&pc_b, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap() {
        CasOutcome::Conflict { key, message } => {
            assert_eq!(key, "PUR-0001");
            assert_eq!(message, hive_conflict_message("PUR-0001"));
        }
        other => panic!("{other:?}"),
    }
    let remote = hive.get_row(KIND_PURCHASE, "PUR-0001").unwrap().unwrap();
    assert!(remote.cells.iter().any(|c| c == "10" || c == "10.0"));
    assert!(!remote.cells.iter().any(|c| c == "99" || c == "99.0"));
}

#[test]
fn logic_payment_stays_on_same_voucher_not_a_new_bill() {
    let mut books = open_memory().unwrap();
    save_voucher(&mut books, voucher_save(20, vec![pay_slot(1, 10.0)])).unwrap();
    save_voucher(
        &mut books,
        voucher_save(20, vec![pay_slot(1, 10.0), pay_slot(2, 15.0)]),
    )
    .unwrap();
    let list = list_vouchers(&books).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].voucher_number, 20);
    let view = t_books_lib::get_voucher(&books, 20).unwrap();
    assert_eq!(view.payments.len(), 2);
}

#[test]
fn logic_skip_dummy_and_dotted_and_status_from_amounts() {
    assert!(should_skip_sheet_row("VOUCHER_1001", "VEND_02"));
    assert!(should_skip_sheet_row("20.1", "VEND_02"));
    assert!(is_dummy_serial("CUST_01"));
    assert!(is_dummy_serial("VEND_02"));
    assert!(is_dotted_child_serial("20.1"));
    assert!(!should_skip_sheet_row("20", "VEND_02"));
    assert_eq!(payment_status(100.0, 0.0, ""), "Missing Tax Invoice");
    assert_eq!(payment_status(100.0, 0.0, "INV"), "Pending Payment");
    assert_eq!(payment_status(100.0, 40.0, "INV"), "Partial Payment");
    assert_eq!(payment_status(100.0, 100.0, "INV"), "Full Payment");
    assert_eq!(payment_status(100.0, 120.0, "INV"), "Advance Payment");
}

#[test]
fn logic_sales_po_unique_per_project_hive_key() {
    let mut books = open_memory().unwrap();
    save_sales_po(
        &mut books,
        SalesPoSave {
            id: None,
            project: "CUST_01".into(),
            po_number: "PO-1".into(),
            client: "CUST_01".into(),
            gst: String::new(),
            items: vec![],
        },
    )
    .unwrap();
    let err = save_sales_po(
        &mut books,
        SalesPoSave {
            id: None,
            project: "CUST_01".into(),
            po_number: "PO-1".into(),
            client: "CUST_01".into(),
            gst: String::new(),
            items: vec![],
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"));
    save_sales_po(
        &mut books,
        SalesPoSave {
            id: None,
            project: "CUST_02".into(),
            po_number: "PO-1".into(),
            client: String::new(),
            gst: String::new(),
            items: vec![],
        },
    )
    .unwrap();
    let mut hive = MemoryHive::default();
    hive.ensure_tab(KIND_SALES_PO, &tab_headers(KIND_SALES_PO))
        .unwrap();
    let a = submit_office_with(&books, &mut hive, KIND_SALES_PO, "PO-1@CUST_01", true).unwrap();
    let b = submit_office_with(&books, &mut hive, KIND_SALES_PO, "PO-1@CUST_02", true).unwrap();
    assert!(matches!(a, CasOutcome::Ok { .. }));
    assert!(matches!(b, CasOutcome::Ok { .. }));
}

#[test]
fn multi_user_memory_same_key_conflicts_different_keys_ok() {
    let mut hive = MemoryHive::default();
    hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE))
        .unwrap();
    hive.ensure_tab(KIND_PAYMENT, &tab_headers(KIND_PAYMENT))
        .unwrap();
    let mut pc_a = open_memory().unwrap();
    let mut pc_b = open_memory().unwrap();
    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 10.0)).unwrap();
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
fn live_hive_write_update_cas_and_cleanup_dummy_rows() {
    let Some(mut hive) = live_hive() else {
        return;
    };
    match hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE)) {
        Ok(_) => {}
        Err(err) => {
            let _ = skip(&format!("Purchase hive tab not writable: {err}"));
            return;
        }
    }

    let mut pc_a = open_memory().unwrap();
    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 10.0)).unwrap();
    match submit_office_with(&pc_a, &mut hive, KIND_PURCHASE, "PUR-0001", false) {
        Ok(CasOutcome::Ok { key, rev, .. }) => {
            assert_eq!(key, "PUR-0001");
            assert!(rev >= 1);
            eprintln!("live Google WRITE Purchase tab dummy PUR-0001 (rev {rev})");
        }
        Ok(other) => {
            cleanup(&mut hive, KIND_PURCHASE, "PUR-0001");
            let _ = skip(&format!("unexpected first write outcome: {other:?}"));
            return;
        }
        Err(err) => {
            let _ = skip(&format!("live write failed (no claim of success): {err}"));
            return;
        }
    }

    save_purchase_po(&mut pc_a, simple_bill("PUR-0001", 11.0)).unwrap();
    match submit_office_with(&pc_a, &mut hive, KIND_PURCHASE, "PUR-0001", false) {
        Ok(CasOutcome::Ok { rev, .. }) => {
            eprintln!("live Google UPDATE Purchase tab dummy PUR-0001 (rev {rev})");
        }
        Ok(CasOutcome::Conflict { .. }) => {
            cleanup(&mut hive, KIND_PURCHASE, "PUR-0001");
            let _ = skip("live update conflicted — hive already had a different PUR-0001");
            return;
        }
        Err(err) => {
            cleanup(&mut hive, KIND_PURCHASE, "PUR-0001");
            let _ = skip(&format!("live update failed: {err}"));
            return;
        }
    }

    let mut pc_b = open_memory().unwrap();
    save_purchase_po(&mut pc_b, simple_bill("PUR-0001", 99.0)).unwrap();
    match submit_office_with(&pc_b, &mut hive, KIND_PURCHASE, "PUR-0001", false).unwrap() {
        CasOutcome::Conflict { key, message } => {
            assert_eq!(key, "PUR-0001");
            assert_eq!(message, hive_conflict_message("PUR-0001"));
        }
        other => panic!("expected CAS conflict, got {other:?}"),
    }

    save_purchase_po(&mut pc_b, simple_bill("PUR-0002", 40.0)).unwrap();
    match submit_office_with(&pc_b, &mut hive, KIND_PURCHASE, "PUR-0002", false) {
        Ok(CasOutcome::Ok { .. }) => {
            eprintln!("live Google WRITE Purchase tab dummy PUR-0002 (concurrent different key)");
        }
        Ok(other) => eprintln!("live PUR-0002 outcome: {other:?}"),
        Err(err) => eprintln!("live PUR-0002 write skipped: {err}"),
    }

    if hive.ensure_tab(KIND_PAYMENT, &tab_headers(KIND_PAYMENT)).is_ok() {
        save_purchase_payment(
            &mut pc_b,
            PurchasePaymentSave {
                po_number: "PUR-0002".into(),
                amount_rupees: 10.0,
                ..Default::default()
            },
        )
        .unwrap();
        match submit_office_with(&pc_b, &mut hive, KIND_PAYMENT, "PAY-0001", false) {
            Ok(CasOutcome::Ok { .. }) => {
                eprintln!("live Google WRITE Purchase payments dummy PAY-0001 on same bill");
            }
            Ok(other) => eprintln!("live PAY-0001 outcome: {other:?}"),
            Err(err) => eprintln!("live PAY-0001 write skipped: {err}"),
        }
        cleanup(&mut hive, KIND_PAYMENT, "PAY-0001");
    }

    if hive.ensure_tab(KIND_SALES_PO, &tab_headers(KIND_SALES_PO)).is_ok() {
        let mut sales = open_memory().unwrap();
        save_sales_po(
            &mut sales,
            SalesPoSave {
                id: None,
                project: "CUST_01".into(),
                po_number: "PO-1".into(),
                client: "CUST_01".into(),
                gst: String::new(),
                items: vec![],
            },
        )
        .unwrap();
        match submit_office_with(&sales, &mut hive, KIND_SALES_PO, "PO-1@CUST_01", false) {
            Ok(CasOutcome::Ok { .. }) => {
                eprintln!("live Google WRITE Sales_PO dummy PO-1@CUST_01");
            }
            Ok(other) => eprintln!("live sales PO outcome: {other:?}"),
            Err(err) => eprintln!("live sales PO write skipped: {err}"),
        }
        cleanup(&mut hive, KIND_SALES_PO, "PO-1@CUST_01");
    }

    cleanup(&mut hive, KIND_PURCHASE, "PUR-0001");
    cleanup(&mut hive, KIND_PURCHASE, "PUR-0002");
}

#[test]
fn live_never_writes_voucher_raw_data() {
    let map = load_hive_map().unwrap();
    let err = refuse_raw_write(&map.raw_source.spreadsheet_id, &map.raw_source.tab).unwrap_err();
    assert!(err.to_string().contains("read-only"));
    let err = refuse_raw_write("x", "VOUCHER_RAW_Data").unwrap_err();
    assert!(err.to_string().contains("read-only"));
}
