//! Remove a parked row from Google so other PCs do not get it back on Refresh.
use crate::db::LocalBooks;
use crate::hive::{
    bump_pending_attempt, clear_pending, enqueue_submit, fingerprint_cells, CasOutcome, Hive,
    KIND_DOCUMENT, KIND_INVENTORY, KIND_LOGISTICS, KIND_PAYMENT, KIND_PURCHASE, KIND_SALARY,
    KIND_SALES_PO, KIND_VOUCHER,
};
use crate::office_sync::GoogleOfficeHive;
use crate::online::is_online;
use crate::sheets::credentials_exist;
use crate::submit::{GoogleSheet, VoucherSheet};
use crate::{BooksError, Result};

const ERASE_PAYLOAD: &str = r#"{\"op\":\"erase\"}"#;

pub fn erase_hive_row(books: &LocalBooks, kind: &str, key: &str) -> Result<()> {
    let key = key.trim();
    if key.is_empty() {
        return Ok(());
    }
    enqueue_submit(books, key, kind, ERASE_PAYLOAD, "", 0)?;
    if !is_online() || !credentials_exist() {
        return Ok(());
    }
    match erase_live(kind, key) {
        Ok(()) => clear_pending(books, key, kind),
        Err(err) => {
            let _ = bump_pending_attempt(books, key, kind, &err.to_string());
            Err(err)
        }
    }
}

/// Best-effort. Local park already happened.
pub fn erase_hive_row_quiet(books: &LocalBooks, kind: &str, key: &str) {
    let _ = erase_hive_row(books, kind, key);
}

fn erase_live(kind: &str, key: &str) -> Result<()> {
    if kind == KIND_VOUCHER {
        return erase_voucher_register(key);
    }
    let mut hive = GoogleOfficeHive::connect()?;
    erase_on_hive(&mut hive, kind, key)
}

fn erase_voucher_register(key: &str) -> Result<()> {
    let n: i64 = key
        .parse()
        .map_err(|_| BooksError::from("That voucher number is not valid."))?;
    let mut sheet = GoogleSheet::connect()?;
    let hits = sheet.lookup(n)?;
    if hits.is_empty() {
        return Ok(());
    }
    for hit in hits {
        let blanks = vec![String::new(); hit.cells.len().max(1)];
        sheet.update_row(hit.sheet_row, &blanks)?;
    }
    Ok(())
}

pub fn erase_on_hive(hive: &mut dyn Hive, kind: &str, key: &str) -> Result<()> {
    let Some(row) = hive.get_row(kind, key)? else {
        return Ok(());
    };
    let blanks = vec![String::new(); row.cells.len().max(1)];
    let new_fp = fingerprint_cells(&blanks);
    match hive.cas_row(kind, key, &row.fp, row.rev, &blanks, &new_fp)? {
        CasOutcome::Ok { .. } => Ok(()),
        CasOutcome::Conflict { .. } => {
            let Some(again) = hive.get_row(kind, key)? else {
                return Ok(());
            };
            let blanks = vec![String::new(); again.cells.len().max(1)];
            let new_fp = fingerprint_cells(&blanks);
            match hive.cas_row(kind, key, &again.fp, again.rev, &blanks, &new_fp)? {
                CasOutcome::Ok { .. } => Ok(()),
                CasOutcome::Conflict { message, .. } => Err(BooksError::from(message)),
            }
        }
    }
}

pub fn kind_for_parked(kind: &str, key: &str) -> (&'static str, String) {
    match kind {
        "voucher" => (KIND_VOUCHER, key.to_string()),
        "purchase" => (KIND_PURCHASE, key.to_string()),
        "sales_po" => (KIND_SALES_PO, key.to_string()),
        "payment" => (KIND_PAYMENT, key.to_string()),
        "salary" => (KIND_SALARY, key.to_string()),
        "inventory" => (KIND_INVENTORY, format!("INV-{key}")),
        "logistics" => (KIND_LOGISTICS, format!("TRIP-{key}")),
        "document" => (KIND_DOCUMENT, format!("DOC-{key}")),
        _ => (KIND_VOUCHER, key.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hive::{tab_headers, MemoryHive};

    #[test]
    fn blank_row_leaves_hive() {
        let headers = tab_headers(KIND_PURCHASE);
        let names: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
        let mut hive = MemoryHive::with_tab(KIND_PURCHASE, &names);
        let cells = vec!["PUR-0001".into(), "VEND_02".into(), "100".into()];
        let fp = fingerprint_cells(&cells);
        hive.cas_row(KIND_PURCHASE, "PUR-0001", "", 0, &cells, &fp).unwrap();
        assert!(hive.get_row(KIND_PURCHASE, "PUR-0001").unwrap().is_some());
        erase_on_hive(&mut hive, KIND_PURCHASE, "PUR-0001").unwrap();
        let left = hive.get_row(KIND_PURCHASE, "PUR-0001").unwrap();
        assert!(left.is_none() || left.unwrap().cells.iter().all(|c| c.trim().is_empty()));
    }
}
