//! Drop local drafts that were never posted. Hive rows are not touched.
use rusqlite::params;

use crate::db::LocalBooks;
use crate::hive::{list_dirty_keys, DirtyKey, KIND_DOCUMENT, KIND_INVENTORY, KIND_LOGISTICS, KIND_PAYMENT, KIND_PURCHASE, KIND_SALARY, KIND_SALES_PO, KIND_VOUCHER};
use crate::Result;

fn clear_pending(books: &LocalBooks, kind: &str, key: &str) -> Result<()> {
    books.conn().execute(
        "DELETE FROM pending_submit WHERE kind = ?1 AND key = ?2",
        params![kind, key],
    )?;
    Ok(())
}

pub fn discard_dirty_key(books: &LocalBooks, kind: &str, key: &str) -> Result<()> {
    let kind = kind.trim();
    let key = key.trim();
    if kind.is_empty() || key.is_empty() {
        return Ok(());
    }
    match kind {
        k if k == KIND_VOUCHER => {
            let _ = books.conn().execute(
                "DELETE FROM voucher_payments WHERE CAST(voucher_number AS TEXT) = ?1",
                params![key],
            );
            books.conn().execute(
                "DELETE FROM vouchers WHERE CAST(voucher_number AS TEXT) = ?1 AND COALESCE(is_dirty, dirty, 0) = 1",
                params![key],
            )?;
        }
        k if k == KIND_PURCHASE => {
            books.conn().execute(
                "DELETE FROM purchase_po WHERE po_number = ?1 AND COALESCE(is_dirty,0) = 1",
                params![key],
            )?;
        }
        k if k == KIND_PAYMENT => {
            books.conn().execute(
                "DELETE FROM purchase_payments WHERE pay_number = ?1 AND COALESCE(is_dirty,0) = 1",
                params![key],
            )?;
        }
        k if k == KIND_SALES_PO => {
            let _ = books.conn().execute(
                "DELETE FROM sales_po WHERE po_number = ?1 AND COALESCE(is_dirty,0) = 1",
                params![key],
            );
        }
        k if k == KIND_SALARY => {
            books.conn().execute(
                "DELETE FROM hr_payroll WHERE salary_number = ?1 AND COALESCE(is_dirty,0) = 1",
                params![key],
            )?;
        }
        k if k == KIND_INVENTORY => {
            let id = key.trim_start_matches("INV-");
            books.conn().execute(
                "DELETE FROM inventory WHERE CAST(id AS TEXT) = ?1 AND COALESCE(is_dirty,0) = 1",
                params![id],
            )?;
        }
        k if k == KIND_LOGISTICS => {
            let id = key.trim_start_matches("TRIP-");
            books.conn().execute(
                "DELETE FROM logistics WHERE CAST(id AS TEXT) = ?1 AND COALESCE(is_dirty,0) = 1",
                params![id],
            )?;
        }
        k if k == KIND_DOCUMENT => {
            let id = key.trim_start_matches("DOC-");
            books.conn().execute(
                "DELETE FROM documents WHERE CAST(id AS TEXT) = ?1 AND COALESCE(is_dirty,0) = 1",
                params![id],
            )?;
        }
        _ => {}
    }
    clear_pending(books, kind, key)?;
    Ok(())
}

pub fn discard_all_dirty(books: &LocalBooks) -> Result<Vec<DirtyKey>> {
    let keys = list_dirty_keys(books)?;
    for k in &keys {
        discard_dirty_key(books, &k.kind, &k.key)?;
    }
    Ok(keys)
}
