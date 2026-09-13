//! Production-release matrix: every module save/list + hive submit kinds.
//! Dummy tokens only. Views stay local; Submit uses MemoryHive.

use t_books_lib::business_rules::{require_write, amount_or_zero};
use t_books_lib::{
    apply_voucher_rows, backup_to, build_trial, dirty_guard, explore_table, get_voucher_summary,
    inspect_email, list_documents, list_duplicates, list_hr_people, list_hr_payroll, list_inventory,
    list_logistics, list_projects, list_rules, list_vendors, list_vouchers, mark_dirty, open_memory,
    provision_hive, restore_from, save_document, save_hr_payroll,
    save_hr_person, save_inventory, save_logistics, save_purchase_po, save_sales_po, save_voucher,
    search_office, set_first_password, sign_in, submit_office_with, tab_headers, testdata,
    write_access_row, AccessWrite, AuthInspect, CasOutcome, DocumentRow, Hive, HrPerson,
    InventoryRow, LogisticsRow, MemoryHive, PayrollRow, PaymentSave, PurchasePoSave, SalesPoSave,
    VoucherSave, KIND_DOCUMENT, KIND_INVENTORY, KIND_LOGISTICS, KIND_PAYMENT, KIND_PURCHASE,
    KIND_SALARY, KIND_SALES_PO, OWNER_EMAIL, VOUCHER_RAW_TAB,
};

fn inv(name: &str) -> InventoryRow {
    InventoryRow {
        id: None,
        item_name: name.into(),
        item_type: "cable".into(),
        size: "10mm".into(),
        quantity: 2.0,
        cost: 1.0,
        project: "CUST_01".into(),
    }
}

fn logi(vehicle: &str) -> LogisticsRow {
    LogisticsRow {
        id: None,
        project: "CUST_01".into(),
        vehicle_number: vehicle.into(),
        invoice_number: "INV-L".into(),
        start_date: "2026-01-01".into(),
        reach_date: String::new(),
    }
}

fn voucher_save(number: i64, vendor: &str) -> VoucherSave {
    VoucherSave {
        voucher_number: number,
        voucher_date: "2026-01-10".into(),
        tax_invoice: "INV-1001".into(),
        vendor: vendor.into(),
        bank: String::new(),
        account_number: String::new(),
        ifsc: String::new(),
        gst: String::new(),
        project: "CUST_01".into(),
        comments: String::new(),
        description: String::new(),
        voucher_type: String::new(),
        payments: vec![PaymentSave {
            slot: 1,
            pi_no: "PI-1".into(),
            pi_date: "2026-01-11".into(),
            pi_value: 10.0,
            paid: 4.0,
            remaining: 6.0,
            payment_date: String::new(),
            remarks: String::new(),
            description: "Block".into(),
            tds: 0.0,
            payment_details: String::new(),
            payment_percent: 40.0,
        }],
    }
}

#[test]
fn login_set_password_and_owner_session() {
    let books = open_memory().unwrap();
    match inspect_email(&books, OWNER_EMAIL) {
        AuthInspect::SetPassword { .. } => {}
        other => panic!("{other:?}"),
    }
    set_first_password(&books, OWNER_EMAIL, "OfficePass1", "OfficePass1").unwrap();
    let session = sign_in(&books, OWNER_EMAIL, "OfficePass1").unwrap();
    assert_eq!(session.role, "owner");
}

#[test]
fn board_counts_and_dirty_guard_are_local() {
    let mut books = open_memory().unwrap();
    apply_voucher_rows(&mut books, &testdata::parse_clean_one_block(3)).unwrap();
    let summary = get_voucher_summary(&books).unwrap();
    assert_eq!(summary.count, 3);
    mark_dirty(&books, 1).unwrap();
    assert_eq!(dirty_guard(&books).unwrap(), Some(vec![1]));
}

#[test]
fn finance_trial_from_local_purchase() {
    let mut books = open_memory().unwrap();
    save_purchase_po(
        &mut books,
        PurchasePoSave {
            project: "CUST_01".into(),
            vendor: "VEND_02".into(),
            po_number: "PUR-0001".into(),
            po_type: "simple".into(),
            total_value: 50.0,
            ..Default::default()
        },
    )
    .unwrap();
    let trial = build_trial(&books, None, None).unwrap();
    assert!(trial.balanced);
}

#[test]
fn every_office_module_saves_locally() {
    let mut books = open_memory().unwrap();
    save_voucher(&mut books, voucher_save(1001, "VEND_02")).unwrap();
    save_sales_po(
        &mut books,
        SalesPoSave {
            id: None,
            project: "CUST_01".into(),
            po_number: "SAL-PO-1".into(),
            client: "CUST_01".into(),
            gst: String::new(),
            items: vec![],
        },
    )
    .unwrap();
    save_purchase_po(
        &mut books,
        PurchasePoSave {
            project: "CUST_01".into(),
            vendor: "VEND_02".into(),
            po_number: "PUR-0001".into(),
            po_type: "simple".into(),
            total_value: 10.0,
            ..Default::default()
        },
    )
    .unwrap();
    let person = save_hr_person(
        &books,
        HrPerson {
            name: "Asha".into(),
            role: "Ops".into(),
            salary: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    save_hr_payroll(
        &books,
        PayrollRow {
            person_id: person.id.unwrap(),
            month: "2026-02".into(),
            pf_employee: 1.0,
            pf_company: 1.0,
            tds: 0.0,
            total_paid: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    save_inventory(&books, inv("Cable")).unwrap();
    save_logistics(&books, logi("MH12AB0000")).unwrap();
    save_document(
        &books,
        DocumentRow {
            id: None,
            name: "note.txt".into(),
            path: "/tmp/tbooks-dummy-note.txt".into(),
            linked_type: "project".into(),
            linked_id: "CUST_01".into(),
            created_at: String::new(),
        },
    )
    .unwrap();

    assert_eq!(list_vouchers(&books).unwrap().len(), 1);
    assert!(!list_vendors(&books).unwrap().is_empty() || !list_projects(&books).unwrap().is_empty());
    assert_eq!(list_hr_people(&books).unwrap().len(), 1);
    assert_eq!(list_hr_payroll(&books).unwrap().len(), 1);
    assert_eq!(list_inventory(&books, None).unwrap().len(), 1);
    assert_eq!(list_logistics(&books).unwrap().len(), 1);
    assert_eq!(list_documents(&books).unwrap().len(), 1);
    let hits = search_office(&books, "CUST").unwrap();
    assert!(hits.iter().any(|h| h.kind == "project" || h.kind == "inventory"));
}

#[test]
fn control_duplicates_explorer_rules_are_local() {
    let books = open_memory().unwrap();
    let _ = list_duplicates(&books).unwrap();
    let _ = explore_table(&books, "access_cache").unwrap();
    let rules = list_rules(&books).unwrap();
    assert!(!rules.is_empty());
}

#[test]
fn submit_office_kinds_cas_on_memory_hive() {
    let mut books = open_memory().unwrap();
    save_purchase_po(
        &mut books,
        PurchasePoSave {
            project: "CUST_01".into(),
            vendor: "VEND_02".into(),
            po_number: "PUR-0001".into(),
            po_type: "simple".into(),
            total_value: 10.0,
            ..Default::default()
        },
    )
    .unwrap();
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
    let person = save_hr_person(
        &books,
        HrPerson {
            name: "Asha".into(),
            salary: 10.0,
            ..Default::default()
        },
    )
    .unwrap();
    let pay = save_hr_payroll(
        &books,
        PayrollRow {
            person_id: person.id.unwrap(),
            month: "2026-03".into(),
            pf_employee: 1.0,
            pf_company: 1.0,
            tds: 0.0,
            total_paid: 8.0,
            ..Default::default()
        },
    )
    .unwrap();
    let inv_row = save_inventory(&books, inv("Reel")).unwrap();
    let log_row = save_logistics(&books, logi("MH12AB0001")).unwrap();
    let doc = save_document(
        &books,
        DocumentRow {
            id: None,
            name: "pack.txt".into(),
            path: "/tmp/pack.txt".into(),
            linked_type: String::new(),
            linked_id: String::new(),
            created_at: String::new(),
        },
    )
    .unwrap();

    let mut hive = MemoryHive::default();
    for kind in [
        KIND_PURCHASE,
        KIND_PAYMENT,
        KIND_SALARY,
        KIND_SALES_PO,
        KIND_INVENTORY,
        KIND_LOGISTICS,
        KIND_DOCUMENT,
    ] {
        hive.ensure_tab(kind, &tab_headers(kind)).unwrap();
    }
    assert!(matches!(
        submit_office_with(&books, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap(),
        CasOutcome::Ok { .. }
    ));
    assert!(matches!(
        submit_office_with(&books, &mut hive, KIND_SALES_PO, "PO-1@CUST_01", true).unwrap(),
        CasOutcome::Ok { .. }
    ));
    assert!(matches!(
        submit_office_with(&books, &mut hive, KIND_SALARY, &pay.salary_number, true).unwrap(),
        CasOutcome::Ok { .. }
    ));
    let inv_key = format!("INV-{}", inv_row.id.unwrap());
    assert!(matches!(
        submit_office_with(&books, &mut hive, KIND_INVENTORY, &inv_key, true).unwrap(),
        CasOutcome::Ok { .. }
    ));
    let log_key = format!("TRIP-{}", log_row.id.unwrap());
    assert!(matches!(
        submit_office_with(&books, &mut hive, KIND_LOGISTICS, &log_key, true).unwrap(),
        CasOutcome::Ok { .. }
    ));
    let doc_key = format!("DOC-{}", doc.id.unwrap());
    assert!(matches!(
        submit_office_with(&books, &mut hive, KIND_DOCUMENT, &doc_key, true).unwrap(),
        CasOutcome::Ok { .. }
    ));
}

#[test]
fn access_write_without_google_is_a_real_error() {
    let mut books = open_memory().unwrap();
    let err = write_access_row(
        &mut books,
        AccessWrite {
            name: "Ada".into(),
            email: "ada@office.test".into(),
            role: "admin".into(),
            active: "Yes".into(),
        },
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("offline") || msg.contains("credentials") || msg.contains("Google"),
        "{msg}"
    );
}

#[test]
fn provision_without_live_google_does_not_panic() {
    let report = provision_hive(false).unwrap();
    assert!(report.error.is_some() || !report.folders.is_empty());
}

#[test]
fn backup_restore_round_trip_vouchers() {
    let dir = testdata::temp_db("rel-bak");
    let mut books = t_books_lib::open_at(&dir).unwrap();
    apply_voucher_rows(&mut books, &testdata::parse_clean_one_block(2)).unwrap();
    let zip = dir.parent().unwrap().join("T-Books-rel.zip");
    backup_to(&books, &zip).unwrap();
    books
        .conn()
        .execute_batch("DELETE FROM voucher_payments; DELETE FROM vouchers;")
        .unwrap();
    restore_from(&mut books, &zip).unwrap();
    assert_eq!(list_vouchers(&books).unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn viewer_cannot_mutate_and_odd_money_is_zero() {
    assert!(require_write("viewer").is_err());
    assert!(require_write("operator").is_ok());
    assert_eq!(amount_or_zero(""), 0.0);
    assert_eq!(amount_or_zero("not-a-number"), 0.0);
    assert_eq!(VOUCHER_RAW_TAB, "Voucher_Raw_Data");
}
