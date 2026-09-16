//! Vendors / project masters and voucher-to-PUR merge. SQLite only.

use rusqlite::params;

use crate::core::business_rules::{
    can_unmerge_voucher, merge_blocked_reason, vendor_merge_key, MergeVoucherHint,
};
use crate::db::LocalBooks;
use crate::{BooksError, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorRow {
    pub vendor: String,
    pub gst: String,
    #[serde(default)]
    pub bank: String,
    #[serde(default)]
    pub account_number: String,
    #[serde(default)]
    pub ifsc: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeReport {
    pub po_number: String,
    pub linked: Vec<i64>,
    pub skipped: Vec<String>,
}

pub fn list_vendors(books: &LocalBooks) -> Result<Vec<VendorRow>> {
    let mut stmt = books.conn().prepare(
        "SELECT vendor,
                COALESCE(gst,''),
                COALESCE(bank,''),
                COALESCE(account_number,''),
                COALESCE(ifsc,'')
         FROM vendors ORDER BY vendor COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(VendorRow {
            vendor: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            gst: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            bank: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            account_number: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            ifsc: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        let v = row?;
        if !v.vendor.trim().is_empty() {
            out.push(v);
        }
    }
    Ok(out)
}

fn load_hint(books: &LocalBooks, voucher_number: i64) -> Result<MergeVoucherHint> {
    let row = books.conn().query_row(
        "SELECT COALESCE(vendor,''), COALESCE(linked_po,''), COALESCE(linked_source,''), COALESCE(source,'')
         FROM vouchers WHERE voucher_number = ?1",
        params![voucher_number],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        },
    );
    let (vendor, linked_po, linked_source, source) = row.map_err(|_| {
        BooksError::from(format!("Voucher {voucher_number} is not on this PC."))
    })?;
    let src = if !linked_source.trim().is_empty() {
        linked_source
    } else {
        source
    };
    Ok(MergeVoucherHint {
        id: voucher_number.to_string(),
        vendor_key: vendor_merge_key(&vendor, None),
        po_id: if linked_po.trim().is_empty() {
            None
        } else {
            Some(linked_po)
        },
        source: Some(src),
        reversed: false,
    })
}

pub fn merge_onto_purchase(
    books: &mut LocalBooks,
    po_number: &str,
    voucher_numbers: &[i64],
) -> Result<MergeReport> {
    let po = po_number.trim();
    if po.is_empty() {
        return Err(BooksError::from("Choose a purchase to merge onto."));
    }
    if po.contains('.') {
        return Err(BooksError::from(
            "Posted child numbers like PUR-0001-01 are never rewritten.",
        ));
    }
    let bill = books
        .conn()
        .query_row(
            "SELECT vendor FROM purchase_po WHERE po_number = ?1 COLLATE NOCASE LIMIT 1",
            params![po],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|_| BooksError::from("That purchase bill was not found on this PC."))?;
    let vendor = bill.unwrap_or_default();
    let target_key = vendor_merge_key(&vendor, None);
    let mut hints = Vec::new();
    for n in voucher_numbers {
        hints.push(load_hint(books, *n)?);
    }
    if let Some(reason) = merge_blocked_reason(&hints, Some(po), Some(&target_key)) {
        return Err(BooksError::from(reason));
    }
    let tx = books.conn_mut().transaction()?;
    let mut linked = Vec::new();
    let mut skipped = Vec::new();
    for n in voucher_numbers {
        let changed = tx.execute(
            "UPDATE vouchers
             SET linked_po = ?1,
                 linked_source = CASE WHEN COALESCE(linked_source,'') = 'import' THEN 'import' ELSE 'purchase' END,
                 is_dirty = 1,
                 dirty = 1,
                 updated_at = datetime('now')
             WHERE voucher_number = ?2",
            params![po, n],
        )?;
        if changed == 0 {
            skipped.push(format!("{n} not on this PC"));
        } else {
            linked.push(*n);
        }
    }
    tx.commit()?;
    Ok(MergeReport {
        po_number: po.to_string(),
        linked,
        skipped,
    })
}

pub fn unmerge_voucher(books: &mut LocalBooks, voucher_number: i64) -> Result<()> {
    let hint = load_hint(books, voucher_number)?;
    if !can_unmerge_voucher(hint.source.as_deref()) {
        return Err(BooksError::from(
            "Unmerge is only for linked import payments.",
        ));
    }
    let n = books.conn().execute(
        "UPDATE vouchers
         SET linked_po = '',
             is_dirty = 1,
             dirty = 1,
             updated_at = datetime('now')
         WHERE voucher_number = ?1",
        params![voucher_number],
    )?;
    if n == 0 {
        return Err(BooksError::from(format!(
            "Voucher {voucher_number} is not on this PC."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    fn seed(books: &mut LocalBooks) {
        books
            .conn()
            .execute(
                "INSERT INTO purchase_po (project, vendor, po_number, type, total_value, is_dirty)
                 VALUES ('CUST_01', 'VEND_02', 'PUR-0001', 'contract', 100, 0)",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vouchers (voucher_number, vendor, project, linked_po, linked_source, source)
                 VALUES (20, 'VEND_02', 'CUST_01', '', 'import', 'import')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vouchers (voucher_number, vendor, project, linked_po, source)
                 VALUES (21, 'OTHER', 'CUST_01', '', 'purchase')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vendors (vendor, bank, account_number, ifsc, gst)
                 VALUES ('VEND_02', 'HDFC', '001', 'HDFC0001', 'GST00')",
                [],
            )
            .unwrap();
    }

    #[test]
    fn vendors_include_bank_line() {
        let mut books = open_memory().unwrap();
        seed(&mut books);
        let rows = list_vendors(&books).unwrap();
        assert_eq!(rows[0].vendor, "VEND_02");
        assert_eq!(rows[0].bank, "HDFC");
        assert_eq!(rows[0].ifsc, "HDFC0001");
    }

    #[test]
    fn merge_same_vendor_and_reject_other() {
        let mut books = open_memory().unwrap();
        seed(&mut books);
        merge_onto_purchase(&mut books, "PUR-0001", &[21]).unwrap_err();
        let ok = merge_onto_purchase(&mut books, "PUR-0001", &[20]).unwrap();
        assert_eq!(ok.linked, vec![20]);
        let linked: String = books
            .conn()
            .query_row(
                "SELECT linked_po FROM vouchers WHERE voucher_number = 20",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(linked, "PUR-0001");
        let dirty: i64 = books
            .conn()
            .query_row(
                "SELECT is_dirty FROM vouchers WHERE voucher_number = 20",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dirty, 1);
    }

    #[test]
    fn unmerge_only_import() {
        let mut books = open_memory().unwrap();
        seed(&mut books);
        merge_onto_purchase(&mut books, "PUR-0001", &[20]).unwrap();
        unmerge_voucher(&mut books, 20).unwrap();
        books
            .conn()
            .execute(
                "UPDATE vouchers SET linked_source = 'purchase', linked_po = 'PUR-0001' WHERE voucher_number = 20",
                [],
            )
            .unwrap();
        unmerge_voucher(&mut books, 20).unwrap_err();
    }

    #[test]
    fn refuses_dotted_child_target() {
        let mut books = open_memory().unwrap();
        seed(&mut books);
        let err = merge_onto_purchase(&mut books, "PUR-0001-01", &[20]).unwrap_err();
        assert!(err.to_string().contains("never rewritten"));
    }
}
