//! A. Functional: login, access, vouchers, payments, status, refresh, dirty guard.

use t_books_lib::business_rules::{
    add_payment, can_add_payment, empty_payment, occupied_payment_count, parse_voucher_number,
    summarize_voucher, PaymentBlock, VoucherInput, MAX_PAYMENT_BLOCKS,
};
use t_books_lib::testdata::{parse_clean_one_block, voucher_row};
use t_books_lib::{
    apply_access_rows, apply_voucher_rows, dirty_guard, inspect_email, list_vouchers, mark_dirty,
    open_memory, parse_voucher_values, refresh_vouchers, set_first_password, sign_in, AccessRow,
    AuthInspect, PoItemIn, SalesPoSave, OWNER_EMAIL,
};
use t_books_lib::{save_sales_po, OFFLINE_BANNER};

fn access(name: &str, email: &str, role: &str, active: &str) -> AccessRow {
    AccessRow {
        name: name.into(),
        email: email.into(),
        role: role.into(),
        active: active.into(),
        last_synced: String::new(),
        account_type: role.to_string(),
    }
}

#[test]
fn owner_sets_password_then_signs_in_offline() {
    let books = open_memory().unwrap();
    match inspect_email(&books, OWNER_EMAIL) {
        AuthInspect::SetPassword { email } => assert_eq!(email, OWNER_EMAIL),
        other => panic!("expected setup, got {other:?}"),
    }
    set_first_password(&books, OWNER_EMAIL, "OfficePass1", "OfficePass1").unwrap();
    let session = sign_in(&books, OWNER_EMAIL, "OfficePass1").unwrap();
    assert_eq!(session.role, "owner");
    let bad = sign_in(&books, OWNER_EMAIL, "wrong-pass").unwrap_err();
    assert!(bad.to_string().contains("Invalid"));
}

#[test]
fn access_roles_owner_admin_operator() {
    let mut books = open_memory().unwrap();
    apply_access_rows(
        &mut books,
        &[
            access("Nitin", OWNER_EMAIL, "owner", "Yes"),
            access("Ada", "ada@office.test", "admin", "Yes"),
            access("Op", "op@office.test", "operator", "Yes"),
            access("Off", "off@office.test", "operator", "No"),
        ],
    )
    .unwrap();
    set_first_password(&books, "ada@office.test", "AdminPass1", "AdminPass1").unwrap();
    set_first_password(&books, "op@office.test", "OperPass1", "OperPass1").unwrap();
    assert_eq!(sign_in(&books, "ada@office.test", "AdminPass1").unwrap().role, "admin");
    assert_eq!(sign_in(&books, "op@office.test", "OperPass1").unwrap().role, "operator");
    match inspect_email(&books, "off@office.test") {
        AuthInspect::Denied { message } => assert!(message.contains("not active")),
        other => panic!("{other:?}"),
    }
    match inspect_email(&books, "nobody@office.test") {
        AuthInspect::Denied { message } => assert!(message.contains("Access list")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn voucher_apply_list_and_status() {
    let mut books = open_memory().unwrap();
    let parsed = parse_voucher_values(&[voucher_row(1001, 0.4)]);
    apply_voucher_rows(&mut books, &parsed).unwrap();
    let list = list_vouchers(&books).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].voucher_number, 1001);
    let computed = summarize_voucher(&VoucherInput {
        voucher_number: 1001,
        tax_invoice: "INV-1001".into(),
        vendor: "VEND".into(),
        bank: String::new(),
        account_number: String::new(),
        ifsc: String::new(),
        gst: String::new(),
        project: "CUST".into(),
        comments: String::new(),
        payments: parsed.vouchers[0].payments.clone(),
    });
    assert!(computed.total_value > 0.0);
    assert!(computed.remaining >= 0.0);
    assert!(!computed.status.is_empty());
}

#[test]
fn payment_blocks_cap_at_five() {
    let mut blocks: Vec<PaymentBlock> = (1..=5).map(empty_payment).collect();
    for b in &mut blocks {
        b.pi_no = "PI".into();
    }
    assert_eq!(occupied_payment_count(&blocks), 5);
    assert!(!can_add_payment(&blocks));
    let err = add_payment(&blocks, empty_payment(6)).unwrap_err();
    assert!(err.contains("5"));
    assert_eq!(MAX_PAYMENT_BLOCKS, 5);
}

#[test]
fn dirty_guard_blocks_refresh() {
    let mut books = open_memory().unwrap();
    apply_voucher_rows(&mut books, &parse_clean_one_block(2)).unwrap();
    mark_dirty(&books, 1).unwrap();
    let dirty = dirty_guard(&books).unwrap().expect("dirty");
    assert_eq!(dirty, vec![1]);
}

#[test]
fn refresh_failure_keeps_local_and_is_a_real_error() {
    let mut books = open_memory().unwrap();
    apply_voucher_rows(&mut books, &parse_clean_one_block(1)).unwrap();
    let err = refresh_vouchers(&mut books).unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty());
    assert_eq!(list_vouchers(&books).unwrap().len(), 1);
}

#[test]
fn sales_po_round_trip_uses_calc_po() {
    let mut books = open_memory().unwrap();
    let saved = save_sales_po(
        &mut books,
        SalesPoSave {
            id: None,
            project: "CUST_01".into(),
            po_number: "PO-1".into(),
            client: "Client".into(),
            gst: String::new(),
            items: vec![PoItemIn {
                id: None,
                item_name: String::new(),
                description: "Line".into(),
                qty: 2.0,
                rate: 100.0,
                gst_pct: 18.0,
                amount: 0.0,
            }],
        },
    )
    .unwrap();
    assert!(saved.total_value > 0.0);
}

#[test]
fn offline_banner_text_is_exact() {
    assert_eq!(
        OFFLINE_BANNER,
        "Offline — working on this PC. Access list and Refresh paused."
    );
}

#[test]
fn column_a_rejects_decimal_string_negative() {
    assert_eq!(parse_voucher_number("20"), Some(20));
    assert_eq!(parse_voucher_number("20.1"), None);
    assert_eq!(parse_voucher_number("ABC"), None);
    assert_eq!(parse_voucher_number("-9"), None);
}
