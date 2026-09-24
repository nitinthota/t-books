//! Extra hive rows that belong to a parent Post.
//! Vendor extra banks, PUR lines, SAL lines. One key = one CAS row.

use crate::db::LocalBooks;
use crate::hive::get_row_rev;
use crate::hive_plan::{KIND_PURCHASE_ITEM, KIND_SALES_ITEM, KIND_VENDOR, KIND_VENDOR_BANK};
use crate::{BooksError, Result};

pub fn child_cells(books: &LocalBooks, kind: &str, key: &str) -> Result<(Vec<String>, String, i64)> {
    match kind {
        KIND_VENDOR => load_vendor(books, key),
        KIND_VENDOR_BANK => load_vendor_bank(books, key),
        KIND_PURCHASE_ITEM => load_purchase_item(books, key),
        KIND_SALES_ITEM => load_sales_item(books, key),
        _ => Err(BooksError::from("That hive kind cannot be submitted.")),
    }
}

pub fn is_child_kind(kind: &str) -> bool {
    matches!(
        kind,
        KIND_VENDOR | KIND_VENDOR_BANK | KIND_PURCHASE_ITEM | KIND_SALES_ITEM
    )
}

fn rupees_cell(n: f64) -> String {
    if n.is_finite() {
        format!("{n:.2}")
    } else {
        "0.00".into()
    }
}

fn load_vendor(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let vendor = key.trim();
    let row = books
        .conn()
        .query_row(
            "SELECT COALESCE(vendor,''), COALESCE(gst,''), COALESCE(bank,''), COALESCE(account_number,''), COALESCE(ifsc,'')
             FROM vendors WHERE vendor = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![vendor],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this computer.")))?;
    let (fp, rev) = get_row_rev(books, KIND_VENDOR, vendor).unwrap_or_else(|_| (String::new(), 0));
    Ok((
        vec![row.0, row.1, row.2, row.3, row.4, String::new(), String::new()],
        fp,
        rev,
    ))
}

fn extra_vendor_banks(books: &LocalBooks, vendor: &str) -> Result<Vec<(String, String, String, String)>> {
    let _ = books.conn().execute_batch(
        "CREATE TABLE IF NOT EXISTS vendor_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vendor TEXT NOT NULL COLLATE NOCASE,
            label TEXT,
            bank TEXT,
            account_number TEXT,
            ifsc TEXT,
            is_primary INTEGER NOT NULL DEFAULT 0
         );",
    );
    let mut stmt = books.conn().prepare(
        "SELECT COALESCE(label,''), COALESCE(bank,''), COALESCE(account_number,''), COALESCE(ifsc,'')
         FROM vendor_accounts WHERE vendor = ?1 COLLATE NOCASE AND is_primary = 0 ORDER BY id",
    )?;
    let mapped = stmt.query_map(rusqlite::params![vendor.trim()], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

fn load_vendor_bank(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let (vendor, idx) = key
        .rsplit_once('#')
        .ok_or_else(|| BooksError::from("Bank key must be vendor#n."))?;
    let n: usize = idx
        .parse()
        .map_err(|_| BooksError::from("Bank key must be vendor#n."))?;
    let extras = extra_vendor_banks(books, vendor)?;
    let row = extras
        .get(n.saturating_sub(1))
        .ok_or_else(|| BooksError::from(format!("{key} is not on this computer.")))?;
    let (fp, rev) = get_row_rev(books, KIND_VENDOR_BANK, key).unwrap_or_else(|_| (String::new(), 0));
    Ok((
        vec![
            vendor.trim().to_string(),
            row.0.clone(),
            row.1.clone(),
            row.2.clone(),
            row.3.clone(),
            "No".into(),
        ],
        fp,
        rev,
    ))
}

fn purchase_item_rows(books: &LocalBooks, po: &str) -> Result<Vec<(String, f64, f64, f64)>> {
    let po_id: i64 = books
        .conn()
        .query_row(
            "SELECT id FROM purchase_po WHERE po_number = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![po.trim()],
            |r| r.get(0),
        )
        .map_err(|_| BooksError::from(format!("{po} is not on this computer.")))?;
    let mut stmt = books.conn().prepare(
        "SELECT COALESCE(description,''), COALESCE(qty,0), COALESCE(rate,0), COALESCE(gst_pct,0)
         FROM purchase_po_items WHERE po_id = ?1 ORDER BY id",
    )?;
    let mapped = stmt.query_map(rusqlite::params![po_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, f64>(2)?,
            r.get::<_, f64>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

fn load_purchase_item(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let (po, idx) = key
        .rsplit_once('#')
        .ok_or_else(|| BooksError::from("Line key must be PUR-n#n."))?;
    let n: usize = idx
        .parse()
        .map_err(|_| BooksError::from("Line key must be PUR-n#n."))?;
    let lines = purchase_item_rows(books, po)?;
    let row = lines
        .get(n.saturating_sub(1))
        .ok_or_else(|| BooksError::from(format!("{key} is not on this computer.")))?;
    let amount = row.1 * row.2 * (1.0 + row.3 / 100.0);
    let (fp, rev) = get_row_rev(books, KIND_PURCHASE_ITEM, key).unwrap_or_else(|_| (String::new(), 0));
    Ok((
        vec![
            po.trim().to_string(),
            n.to_string(),
            row.0.clone(),
            format!("{:.3}", row.1),
            rupees_cell(row.2),
            format!("{:.2}", row.3),
            rupees_cell(amount),
        ],
        fp,
        rev,
    ))
}

fn sales_item_rows(books: &LocalBooks, po: &str, project: &str) -> Result<Vec<(String, f64, f64, f64)>> {
    let po_id: i64 = books
        .conn()
        .query_row(
            "SELECT id FROM sales_po WHERE po_number = ?1 COLLATE NOCASE AND project = ?2 COLLATE NOCASE LIMIT 1",
            rusqlite::params![po.trim(), project.trim()],
            |r| r.get(0),
        )
        .map_err(|_| BooksError::from(format!("{po} is not on this computer.")))?;
    let mut stmt = books.conn().prepare(
        "SELECT COALESCE(description,''), COALESCE(qty,0), COALESCE(rate,0), COALESCE(gst_pct,0)
         FROM sales_po_items WHERE po_id = ?1 ORDER BY id",
    )?;
    let mapped = stmt.query_map(rusqlite::params![po_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, f64>(2)?,
            r.get::<_, f64>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

fn load_sales_item(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let (head, idx) = key
        .rsplit_once('#')
        .ok_or_else(|| BooksError::from("Line key must be PO@job#n."))?;
    let n: usize = idx
        .parse()
        .map_err(|_| BooksError::from("Line key must be PO@job#n."))?;
    let (po, project) = head.split_once('@').unwrap_or((head, ""));
    let lines = sales_item_rows(books, po, project)?;
    let row = lines
        .get(n.saturating_sub(1))
        .ok_or_else(|| BooksError::from(format!("{key} is not on this computer.")))?;
    let amount = row.1 * row.2 * (1.0 + row.3 / 100.0);
    let (fp, rev) = get_row_rev(books, KIND_SALES_ITEM, key).unwrap_or_else(|_| (String::new(), 0));
    Ok((
        vec![
            po.trim().to_string(),
            project.trim().to_string(),
            n.to_string(),
            row.0.clone(),
            format!("{:.3}", row.1),
            rupees_cell(row.2),
            format!("{:.2}", row.3),
            rupees_cell(amount),
        ],
        fp,
        rev,
    ))
}

pub fn related_keys(books: &LocalBooks, kind: &str, key: &str) -> Result<Vec<(String, String)>> {
    Ok(match kind {
        KIND_VENDOR => extra_vendor_banks(books, key)?
            .into_iter()
            .enumerate()
            .map(|(i, _)| (KIND_VENDOR_BANK.to_string(), format!("{}#{}", key.trim(), i + 1)))
            .collect(),
        crate::hive::KIND_PURCHASE => purchase_item_rows(books, key)?
            .into_iter()
            .enumerate()
            .map(|(i, _)| (KIND_PURCHASE_ITEM.to_string(), format!("{}#{}", key.trim(), i + 1)))
            .collect(),
        crate::hive::KIND_SALES_PO => {
            let (po, project) = key.split_once('@').unwrap_or((key, ""));
            sales_item_rows(books, po, project)?
                .into_iter()
                .enumerate()
                .map(|(i, _)| (KIND_SALES_ITEM.to_string(), format!("{}#{}", key.trim(), i + 1)))
                .collect()
        }
        _ => Vec::new(),
    })
}
