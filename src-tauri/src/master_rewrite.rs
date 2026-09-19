//! Rewrite vendor and job master names on this PC.

use rusqlite::params;

use crate::db::LocalBooks;
use crate::Result;

fn plant_vendor(books: &LocalBooks, name: &str) -> Result<()> {
    books.conn().execute(
        "INSERT INTO vendors (vendor, updated_at) VALUES (?1, datetime('now'))
         ON CONFLICT(vendor) DO UPDATE SET updated_at = datetime('now')",
        params![name],
    )?;
    Ok(())
}

fn plant_project(books: &LocalBooks, name: &str) -> Result<()> {
    books.conn().execute(
        "INSERT INTO projects (project, updated_at) VALUES (?1, datetime('now'))
         ON CONFLICT(project) DO UPDATE SET updated_at = datetime('now')",
        params![name],
    )?;
    Ok(())
}

pub fn rewrite_vendor(books: &LocalBooks, keep: &str, absorb: &str) -> Result<()> {
    plant_vendor(books, keep)?;
    books.conn().execute(
        "UPDATE vendors SET
            bank = COALESCE(NULLIF(bank,''), (SELECT bank FROM vendors WHERE vendor = ?2 COLLATE NOCASE)),
            account_number = COALESCE(NULLIF(account_number,''), (SELECT account_number FROM vendors WHERE vendor = ?2 COLLATE NOCASE)),
            ifsc = COALESCE(NULLIF(ifsc,''), (SELECT ifsc FROM vendors WHERE vendor = ?2 COLLATE NOCASE)),
            gst = COALESCE(NULLIF(gst,''), (SELECT gst FROM vendors WHERE vendor = ?2 COLLATE NOCASE)),
            updated_at = datetime('now')
         WHERE vendor = ?1 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "UPDATE vouchers SET vendor = ?1, dirty = 1, is_dirty = 1, updated_at = datetime('now')
         WHERE vendor = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "UPDATE purchase_po SET vendor = ?1 WHERE vendor = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "UPDATE purchase_payments SET vendor = ?1 WHERE vendor = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "UPDATE sales_po SET client = ?1 WHERE client = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "DELETE FROM vendors WHERE vendor = ?1 COLLATE NOCASE AND vendor != ?2 COLLATE NOCASE",
        params![absorb, keep],
    )?;
    Ok(())
}

pub fn rewrite_project(books: &LocalBooks, keep: &str, absorb: &str) -> Result<()> {
    plant_project(books, keep)?;
    books.conn().execute(
        "UPDATE vouchers SET project = ?1, dirty = 1, is_dirty = 1, updated_at = datetime('now')
         WHERE project = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "UPDATE sales_po SET project = ?1 WHERE project = ?2 COLLATE NOCASE
         AND NOT EXISTS (
           SELECT 1 FROM sales_po k WHERE k.project = ?1 COLLATE NOCASE
             AND lower(k.po_number) = lower(sales_po.po_number)
         )",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "DELETE FROM sales_po WHERE project = ?1 COLLATE NOCASE",
        params![absorb],
    )?;
    books.conn().execute(
        "UPDATE purchase_po SET project = ?1 WHERE project = ?2 COLLATE NOCASE
         AND NOT EXISTS (
           SELECT 1 FROM purchase_po k WHERE k.project = ?1 COLLATE NOCASE
             AND lower(k.po_number) = lower(purchase_po.po_number)
         )",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "DELETE FROM purchase_po WHERE project = ?1 COLLATE NOCASE",
        params![absorb],
    )?;
    books.conn().execute(
        "UPDATE inventory SET project = ?1 WHERE project = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "UPDATE logistics SET project = ?1 WHERE project = ?2 COLLATE NOCASE",
        params![keep, absorb],
    )?;
    books.conn().execute(
        "DELETE FROM projects WHERE project = ?1 COLLATE NOCASE AND project != ?2 COLLATE NOCASE",
        params![absorb, keep],
    )?;
    Ok(())
}
