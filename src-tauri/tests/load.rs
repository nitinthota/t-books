//! D. Load: 5 office users, normal open/edit/save/refresh-guard.

use std::time::Instant;
use t_books_lib::testdata::{parse_clean_one_block, temp_db};
use t_books_lib::{
    apply_voucher_rows, get_voucher, get_voucher_summary, list_vouchers, mark_dirty, open_at,
    PoItemIn, SalesPoSave,
};
use t_books_lib::save_sales_po;

#[test]
fn five_users_normal_office_day() {
    let path = temp_db("load");
    let mut books = open_at(&path).unwrap();
    apply_voucher_rows(&mut books, &parse_clean_one_block(300)).unwrap();
    let mut times = Vec::new();
    for user in 0..5 {
        let t = Instant::now();
        let _ = get_voucher_summary(&books).unwrap();
        let list = list_vouchers(&books).unwrap();
        assert_eq!(list.len(), 300);
        let n = ((user * 17) % 300) + 1;
        let open = get_voucher(&books, n as i64).unwrap();
        assert_eq!(open.voucher_number, n as i64);
        mark_dirty(&books, n as i64).unwrap();
        save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: format!("CUST_{user}"),
                po_number: format!("PO-{user}"),
                client: "Client".into(),
                gst: String::new(),
                items: vec![PoItemIn {
                    id: None,
                    item_name: String::new(),
                    description: "Line".into(),
                    qty: 1.0,
                    rate: 10.0,
                    gst_pct: 0.0,
                    amount: 0.0,
                }],
            },
        )
        .unwrap();
        times.push(t.elapsed().as_millis());
    }
    let max = *times.iter().max().unwrap();
    assert!(max < 2_000, "slowest user {max} ms");
    assert_eq!(list_vouchers(&books).unwrap().len(), 300);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
