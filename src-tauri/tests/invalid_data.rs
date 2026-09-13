//! Invalid input: blank vendor, dotted A, dummy serials, short password, duplicates.

use t_books_lib::business_rules::{
    is_dummy_serial, parse_voucher_number, should_skip_sheet_row, amount_or_zero,
};
use t_books_lib::testdata::{mixed_sheet, parse_mixed, voucher_row};
use t_books_lib::{
    apply_voucher_rows, list_vouchers, open_memory, parse_access_values, parse_voucher_values,
    save_hr_person, save_purchase_po, save_sales_po, save_voucher, validate_new_password, HrPerson,
    PaymentSave, PurchasePoSave, SalesPoSave, VoucherSave,
};

fn save(number: i64, vendor: &str) -> VoucherSave {
    VoucherSave {
        voucher_number: number,
        voucher_date: String::new(),
        tax_invoice: String::new(),
        vendor: vendor.into(),
        bank: String::new(),
        account_number: String::new(),
        ifsc: String::new(),
        gst: String::new(),
        project: "CUST_01".into(),
        comments: String::new(),
        description: String::new(),
        voucher_type: String::new(),
        payments: Vec::<PaymentSave>::new(),
    }
}

#[test]
fn blank_vendor_dotted_dummy_and_non_integer_a() {
    assert!(should_skip_sheet_row("20", ""));
    assert!(should_skip_sheet_row("20.1", "VEND_02"));
    assert!(should_skip_sheet_row("VOUCHER_1001", "VEND_02"));
    assert!(is_dummy_serial("VOUCHER_1001"));
    assert!(is_dummy_serial("CUST_01"));
    assert!(is_dummy_serial("VEND_02"));
    assert_eq!(parse_voucher_number("20.1"), None);
    assert_eq!(parse_voucher_number("ABC"), None);
    assert_eq!(parse_voucher_number("-9"), None);

    let parsed = parse_voucher_values(&[
        voucher_row(20, 0.4),
        {
            let mut r = voucher_row(21, 0.4);
            r[0] = "20.1".into();
            r
        },
        {
            let mut r = voucher_row(22, 0.4);
            r[0] = "VOUCHER_1001".into();
            r
        },
        {
            let mut r = voucher_row(23, 0.4);
            r[3] = String::new();
            r
        },
        {
            let mut r = voucher_row(24, 0.4);
            r[0] = "ABC".into();
            r
        },
    ]);
    assert_eq!(parsed.vouchers.len(), 1);
    assert_eq!(parsed.vouchers[0].voucher_number, 20);
    assert!(parsed.skipped >= 3);
}

#[test]
fn save_voucher_rejects_blank_vendor_and_dummy_token() {
    let mut books = open_memory().unwrap();
    let err = save_voucher(&mut books, save(20, "  ")).unwrap_err();
    assert!(err.to_string().contains("Vendor"));
    let err = save_voucher(&mut books, save(0, "VEND_02")).unwrap_err();
    assert!(
        err.to_string().contains("integer") || err.to_string().contains("Dummy"),
        "{}",
        err
    );
}

#[test]
fn duplicate_po_and_short_password() {
    let mut books = open_memory().unwrap();
    save_sales_po(
        &mut books,
        SalesPoSave {
            id: None,
            project: "CUST_01".into(),
            po_number: "PO-1".into(),
            client: String::new(),
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
            client: String::new(),
            gst: String::new(),
            items: vec![],
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"));
    save_purchase_po(
        &mut books,
        PurchasePoSave {
            project: "CUST_01".into(),
            vendor: "VEND_02".into(),
            po_number: "PUR-0001".into(),
            po_type: "simple".into(),
            total_value: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    let err = save_purchase_po(
        &mut books,
        PurchasePoSave {
            project: "CUST_01".into(),
            vendor: "VEND_02".into(),
            po_number: "PUR-0001".into(),
            po_type: "simple".into(),
            total_value: 2.0,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"));
    assert!(validate_new_password("short", None).is_err());
}

#[test]
fn hr_active_requires_salary() {
    let books = open_memory().unwrap();
    let err = save_hr_person(
        &books,
        HrPerson {
            name: "Asha".into(),
            salary: 0.0,
            active: "Yes".into(),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("Salary"));
}

#[test]
fn empty_access_sheet_is_an_error() {
    let err = parse_access_values(&[]).unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("empty"));
}

#[test]
fn odd_money_cells_coerce_to_zero_on_import() {
    assert_eq!(amount_or_zero(""), 0.0);
    assert_eq!(amount_or_zero("—"), 0.0);
    assert_eq!(amount_or_zero("nope"), 0.0);
    let mut row = voucher_row(30, 0.4);
    row[9 + 2] = "odd".into();
    let parsed = parse_voucher_values(&[row]);
    assert_eq!(parsed.vouchers[0].payments[0].pi_value_rupees, Some(0.0));
}

#[test]
fn mixed_thousand_rows_parse_and_apply() {
    let sheet = mixed_sheet(1000);
    assert!(sheet.len() > 900);
    let parsed = parse_mixed(1000);
    assert!(parsed.skipped > 0);
    assert!(!parsed.vouchers.is_empty());
    let mut books = open_memory().unwrap();
    apply_voucher_rows(&mut books, &parsed).unwrap();
    assert_eq!(list_vouchers(&books).unwrap().len(), parsed.vouchers.len());
}
