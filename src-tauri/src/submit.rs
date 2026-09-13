//! One-voucher write-back. Fingerprint mismatch → conflict, never overwrite.

use rusqlite::OptionalExtension;

use crate::core::business_rules::parse_voucher_number;
use crate::db::LocalBooks;
use crate::hive::{
    bump_pending_attempt, clear_pending, enqueue_submit, pending_payload_json, CasOutcome, Hive,
    KIND_VOUCHER,
};
use crate::hive_plan::{load_map, refuse_raw_write, target_for_kind};
use crate::migrate::register_cells_for_voucher;
use crate::office_sync::GoogleOfficeHive;
use crate::online::is_online;
use crate::sheets::{
    append_row_on, credentials_exist, fetch_a1_on, load_service_account, update_row_on,
    ServiceAccount,
};
use crate::vouchers::{
    apply_one_force, fingerprint_parsed, load_local_voucher, mark_submitted, parse_one_voucher_row,
    voucher_to_sheet_row, ParsedVoucher,
};
use crate::{BooksError, Result};

pub const SHEET_LAST_COL: &str = "BL";

pub fn conflict_message(voucher_number: i64) -> String {
    crate::hive::hive_conflict_message(&format!("Voucher {voucher_number}"))
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SubmitOutcome {
    Ok {
        #[serde(rename = "voucherNumber")]
        voucher_number: i64,
    },
    Conflict {
        #[serde(rename = "voucherNumber")]
        voucher_number: i64,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct RemoteMatch {
    pub sheet_row: u32,
    pub cells: Vec<String>,
}

pub trait VoucherSheet {
    fn lookup(&mut self, voucher_number: i64) -> Result<Vec<RemoteMatch>>;
    fn update_row(&mut self, sheet_row: u32, cells: &[String]) -> Result<()>;
    fn append_row(&mut self, cells: &[String]) -> Result<()>;
}

pub struct MemorySheet {
    pub rows: Vec<Vec<String>>,
    pub fail_writes: bool,
}

impl MemorySheet {
    pub fn with_header() -> Self {
        Self {
            rows: vec![vec![
                "voucher_number".into(),
                "date".into(),
                "tax inv".into(),
                "vendor".into(),
            ]],
            fail_writes: false,
        }
    }
}

impl VoucherSheet for MemorySheet {
    fn lookup(&mut self, voucher_number: i64) -> Result<Vec<RemoteMatch>> {
        let mut out = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
            let sheet_row = (i as u32) + 1;
            if parse_voucher_number(row.first().map(|s| s.as_str()).unwrap_or(""))
                == Some(voucher_number)
            {
                out.push(RemoteMatch {
                    sheet_row,
                    cells: row.clone(),
                });
            }
        }
        Ok(out)
    }

    fn update_row(&mut self, sheet_row: u32, cells: &[String]) -> Result<()> {
        if self.fail_writes {
            return Err(BooksError::from("Could not reach Google Sheets from this PC."));
        }
        let idx = sheet_row.saturating_sub(1) as usize;
        if idx >= self.rows.len() {
            return Err(BooksError::from("That sheet row was not found."));
        }
        self.rows[idx] = cells.to_vec();
        Ok(())
    }

    fn append_row(&mut self, cells: &[String]) -> Result<()> {
        if self.fail_writes {
            return Err(BooksError::from("Could not reach Google Sheets from this PC."));
        }
        self.rows.push(cells.to_vec());
        Ok(())
    }
}

#[allow(dead_code)]
pub struct GoogleSheet {
    account: ServiceAccount,
}

impl GoogleSheet {
    #[allow(dead_code)]
    pub fn connect() -> Result<Self> {
        Ok(Self {
            account: load_service_account()?,
        })
    }
}

impl VoucherSheet for GoogleSheet {
    fn lookup(&mut self, voucher_number: i64) -> Result<Vec<RemoteMatch>> {
        let target = target_for_kind(KIND_VOUCHER)?;
        lookup_column_a(&self.account, &target.spreadsheet_id, &target.tab, voucher_number)
    }

    fn update_row(&mut self, sheet_row: u32, cells: &[String]) -> Result<()> {
        let target = target_for_kind(KIND_VOUCHER)?;
        let a1 = format!("A{sheet_row}:{SHEET_LAST_COL}{sheet_row}");
        update_row_on(
            &self.account,
            &target.spreadsheet_id,
            &target.tab,
            &a1,
            cells,
        )
    }

    fn append_row(&mut self, cells: &[String]) -> Result<()> {
        let target = target_for_kind(KIND_VOUCHER)?;
        append_row_on(&self.account, &target.spreadsheet_id, &target.tab, cells)
    }
}

/// Read-only view of the archive tab. Writes are refused before Google is called.
pub struct GoogleRawArchive {
    account: ServiceAccount,
}

impl GoogleRawArchive {
    pub fn connect() -> Result<Self> {
        Ok(Self {
            account: load_service_account()?,
        })
    }
}

fn lookup_column_a(
    account: &ServiceAccount,
    spreadsheet_id: &str,
    tab: &str,
    voucher_number: i64,
) -> Result<Vec<RemoteMatch>> {
    let column_a = fetch_a1_on(account, spreadsheet_id, tab, "A:A")?;
    let mut hits = Vec::new();
    for (i, row) in column_a.iter().enumerate() {
        let sheet_row = (i as u32) + 1;
        let a = row.first().map(|s| s.as_str()).unwrap_or("");
        if parse_voucher_number(a) == Some(voucher_number) {
            hits.push(sheet_row);
        }
    }
    let mut out = Vec::with_capacity(hits.len());
    for sheet_row in hits {
        let a1 = format!("A{sheet_row}:{SHEET_LAST_COL}{sheet_row}");
        let values = fetch_a1_on(account, spreadsheet_id, tab, &a1)?;
        let cells = values.into_iter().next().unwrap_or_default();
        out.push(RemoteMatch { sheet_row, cells });
    }
    Ok(out)
}

impl VoucherSheet for GoogleRawArchive {
    fn lookup(&mut self, voucher_number: i64) -> Result<Vec<RemoteMatch>> {
        let map = load_map()?;
        lookup_column_a(
            &self.account,
            &map.raw_source.spreadsheet_id,
            &map.raw_source.tab,
            voucher_number,
        )
    }

    fn update_row(&mut self, _sheet_row: u32, _cells: &[String]) -> Result<()> {
        let map = load_map()?;
        refuse_raw_write(&map.raw_source.spreadsheet_id, &map.raw_source.tab)
    }

    fn append_row(&mut self, _cells: &[String]) -> Result<()> {
        let map = load_map()?;
        refuse_raw_write(&map.raw_source.spreadsheet_id, &map.raw_source.tab)
    }
}

fn require_online_for_submit() -> Result<()> {
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot submit to Google.",
        ));
    }
    if !credentials_exist() {
        return Err(BooksError::from(
            "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
        ));
    }
    Ok(())
}

pub fn plan_and_write(
    local: &ParsedVoucher,
    local_source_hash: &str,
    sheet: &mut dyn VoucherSheet,
) -> Result<SubmitOutcome> {
    let n = local.voucher_number;
    if n <= 0 {
        return Err(BooksError::from("Voucher number must be a positive integer."));
    }
    let remote = sheet.lookup(n)?;
    if remote.len() > 1 {
        return Err(BooksError::from(format!(
            "Voucher {n} appears more than once in the sheet."
        )));
    }
    let cells = voucher_to_sheet_row(local);
    let new_hash = fingerprint_parsed(local);
    if remote.is_empty() {
        sheet.append_row(&cells)?;
        return Ok(SubmitOutcome::Ok { voucher_number: n });
    }
    let row = &remote[0];
    let Some(google) = parse_one_voucher_row(&row.cells) else {
        return Err(BooksError::from(format!(
            "Voucher {n} on the sheet has an invalid voucher number."
        )));
    };
    let google_hash = fingerprint_parsed(&google);
    if local_source_hash.is_empty() || local_source_hash != google_hash {
        crate::log::event(
            crate::log::Level::Warn,
            "submit",
            "conflict",
            Some(n),
            "conflict",
        );
        return Ok(SubmitOutcome::Conflict {
            voucher_number: n,
            message: conflict_message(n),
        });
    }
    sheet.update_row(row.sheet_row, &cells)?;
    let _ = new_hash;
    Ok(SubmitOutcome::Ok { voucher_number: n })
}

pub fn submit_voucher_with(
    books: &mut LocalBooks,
    voucher_number: i64,
    sheet: &mut dyn VoucherSheet,
) -> Result<SubmitOutcome> {
    if voucher_number <= 0 {
        return Err(BooksError::from("Voucher number must be a positive integer."));
    }
    let local = load_local_voucher(books, voucher_number)?;
    let new_hash = fingerprint_parsed(&local.parsed);
    let key = voucher_number.to_string();
    let base_rev = crate::hive::get_hive_rev(books, voucher_number).unwrap_or(0);
    enqueue_submit(
        books,
        &key,
        KIND_VOUCHER,
        &pending_payload_json(KIND_VOUCHER, &key),
        &local.source_hash,
        base_rev,
    )?;
    match plan_and_write(&local.parsed, &local.source_hash, sheet) {
        Ok(SubmitOutcome::Ok { voucher_number: n }) => {
            if let Err(err) = mark_submitted(books, n, &new_hash) {
                crate::log::event(
                    crate::log::Level::Error,
                    "submit",
                    "write",
                    Some(n),
                    "local_update_failed",
                );
                return Err(err);
            }
            let _ = clear_pending(books, &key, KIND_VOUCHER);
            crate::log::event(
                crate::log::Level::Info,
                "submit",
                "write",
                Some(n),
                "success",
            );
            Ok(SubmitOutcome::Ok { voucher_number: n })
        }
        Ok(conflict) => Ok(conflict),
        Err(err) => {
            let _ = bump_pending_attempt(books, &key, KIND_VOUCHER, &err.to_string());
            crate::log::event(
                crate::log::Level::Error,
                "submit",
                "write",
                Some(voucher_number),
                "error",
            );
            Err(err)
        }
    }
}

pub fn submit_voucher_hive(
    books: &mut LocalBooks,
    voucher_number: i64,
    hive: &mut dyn Hive,
) -> Result<SubmitOutcome> {
    if voucher_number <= 0 {
        return Err(BooksError::from("Voucher number must be a positive integer."));
    }
    let local = load_local_voucher(books, voucher_number)?;
    let new_hash = fingerprint_parsed(&local.parsed);
    let key = voucher_number.to_string();
    let base_rev = crate::hive::get_hive_rev(books, voucher_number).unwrap_or(0);
    let cells = register_cells_for_voucher(&local.parsed);
    enqueue_submit(
        books,
        &key,
        KIND_VOUCHER,
        &pending_payload_json(KIND_VOUCHER, &key),
        &local.source_hash,
        base_rev,
    )?;
    match crate::hive::submit_hive_row(
        books,
        hive,
        KIND_VOUCHER,
        &key,
        &local.source_hash,
        base_rev,
        &cells,
        &new_hash,
        &pending_payload_json(KIND_VOUCHER, &key),
    ) {
        Ok(CasOutcome::Ok { .. }) => {
            if let Err(err) = mark_submitted(books, voucher_number, &new_hash) {
                crate::log::event(
                    crate::log::Level::Error,
                    "submit",
                    "write",
                    Some(voucher_number),
                    "local_update_failed",
                );
                return Err(err);
            }
            crate::log::event(
                crate::log::Level::Info,
                "submit",
                "write",
                Some(voucher_number),
                "success",
            );
            Ok(SubmitOutcome::Ok { voucher_number })
        }
        Ok(CasOutcome::Conflict { message, .. }) => Ok(SubmitOutcome::Conflict {
            voucher_number,
            message,
        }),
        Err(err) => {
            let _ = bump_pending_attempt(books, &key, KIND_VOUCHER, &err.to_string());
            crate::log::event(
                crate::log::Level::Error,
                "submit",
                "write",
                Some(voucher_number),
                "error",
            );
            Err(err)
        }
    }
}

pub fn submit_voucher(books: &mut LocalBooks, voucher_number: i64) -> Result<SubmitOutcome> {
    require_online_for_submit()?;
    let mut hive = GoogleOfficeHive::connect()?;
    submit_voucher_hive(books, voucher_number, &mut hive)
}

pub fn reload_voucher_with(
    books: &mut LocalBooks,
    voucher_number: i64,
    sheet: &mut dyn VoucherSheet,
) -> Result<ParsedVoucher> {
    let remote = sheet.lookup(voucher_number)?;
    if remote.is_empty() {
        return Err(BooksError::from(format!(
            "Voucher {voucher_number} is not on the sheet."
        )));
    }
    if remote.len() > 1 {
        return Err(BooksError::from(format!(
            "Voucher {voucher_number} appears more than once in the sheet."
        )));
    }
    let Some(parsed) = parse_one_voucher_row(&remote[0].cells) else {
        return Err(BooksError::from(format!(
            "Voucher {voucher_number} on the sheet has an invalid voucher number."
        )));
    };
    apply_one_force(books, &parsed)?;
    crate::log::event(
        crate::log::Level::Info,
        "submit",
        "reload",
        Some(voucher_number),
        "success",
    );
    Ok(parsed)
}

pub fn reload_voucher(books: &mut LocalBooks, voucher_number: i64) -> Result<crate::VoucherView> {
    require_online_for_submit()?;
    let mut sheet = GoogleRawArchive::connect()?;
    reload_voucher_with(books, voucher_number, &mut sheet)?;
    crate::get_voucher(books, voucher_number)
}

#[allow(dead_code)]
pub fn is_dirty_flag(books: &LocalBooks, voucher_number: i64) -> Result<bool> {
    let flag: Option<i64> = books
        .conn()
        .query_row(
            "SELECT COALESCE(is_dirty, dirty, 0) FROM vouchers WHERE voucher_number = ?1",
            rusqlite::params![voucher_number],
            |row| row.get(0),
        )
        .optional()?;
    Ok(flag.unwrap_or(0) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata::{parse_clean_one_block, voucher_row};
    use crate::{apply_voucher_rows, mark_dirty, open_memory, parse_voucher_values};

    fn seed_local(n: i64) -> (crate::LocalBooks, MemorySheet) {
        let mut books = open_memory().unwrap();
        let row = voucher_row(n, 0.4);
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row.clone()])).unwrap();
        let mut sheet = MemorySheet::with_header();
        sheet.rows.push(row);
        (books, sheet)
    }

    #[test]
    fn same_data_write_succeeds() {
        let (mut books, mut sheet) = seed_local(1001);
        let out = submit_voucher_with(&mut books, 1001, &mut sheet).unwrap();
        assert_eq!(out, SubmitOutcome::Ok { voucher_number: 1001 });
        assert!(!is_dirty_flag(&books, 1001).unwrap());
    }

    #[test]
    fn modified_remote_conflicts() {
        let (mut books, mut sheet) = seed_local(1001);
        sheet.rows[1][3] = "OTHER_VENDOR".into();
        match submit_voucher_with(&mut books, 1001, &mut sheet).unwrap() {
            SubmitOutcome::Conflict {
                voucher_number,
                message,
            } => {
                assert_eq!(voucher_number, 1001);
                assert_eq!(message, conflict_message(1001));
            }
            other => panic!("{other:?}"),
        }
        mark_dirty(&books, 1001).unwrap();
        assert!(is_dirty_flag(&books, 1001).unwrap());
    }

    #[test]
    fn multi_pc_second_submit_conflicts() {
        let (mut pc_a, mut sheet) = seed_local(1001);
        let mut pc_b = open_memory().unwrap();
        apply_voucher_rows(&mut pc_b, &parse_voucher_values(&[voucher_row(1001, 0.4)])).unwrap();
        pc_a.conn()
            .execute(
                "UPDATE vouchers SET vendor = 'PC_A' WHERE voucher_number = 1001",
                [],
            )
            .unwrap();
        submit_voucher_with(&mut pc_a, 1001, &mut sheet).unwrap();
        pc_b.conn()
            .execute(
                "UPDATE vouchers SET vendor = 'PC_B' WHERE voucher_number = 1001",
                [],
            )
            .unwrap();
        mark_dirty(&pc_b, 1001).unwrap();
        match submit_voucher_with(&mut pc_b, 1001, &mut sheet).unwrap() {
            SubmitOutcome::Conflict { voucher_number, .. } => assert_eq!(voucher_number, 1001),
            other => panic!("{other:?}"),
        }
        assert!(is_dirty_flag(&pc_b, 1001).unwrap());
        assert_eq!(sheet.rows[1][3], "PC_A");
    }

    #[test]
    fn network_failure_keeps_dirty() {
        let (mut books, mut sheet) = seed_local(1001);
        mark_dirty(&books, 1001).unwrap();
        sheet.fail_writes = true;
        let err = submit_voucher_with(&mut books, 1001, &mut sheet).unwrap_err();
        assert!(!err.to_string().is_empty());
        assert!(is_dirty_flag(&books, 1001).unwrap());
    }

    #[test]
    fn invalid_voucher_rejected() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_clean_one_block(1)).unwrap();
        let mut sheet = MemorySheet::with_header();
        let err = submit_voucher_with(&mut books, 0, &mut sheet).unwrap_err();
        assert!(err.to_string().contains("positive"));
        let missing = submit_voucher_with(&mut books, 99, &mut sheet).unwrap_err();
        assert!(missing.to_string().contains("not on this PC"));
    }

    #[test]
    fn append_when_missing_on_sheet() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[voucher_row(55, 0.2)])).unwrap();
        let mut sheet = MemorySheet::with_header();
        let out = submit_voucher_with(&mut books, 55, &mut sheet).unwrap();
        assert_eq!(out, SubmitOutcome::Ok { voucher_number: 55 });
        assert_eq!(sheet.rows.len(), 2);
        assert_eq!(sheet.rows[1][0], "55");
    }

    #[test]
    fn duplicate_column_a_is_rejected() {
        let (mut books, mut sheet) = seed_local(20);
        sheet.rows.push(voucher_row(20, 0.9));
        let err = submit_voucher_with(&mut books, 20, &mut sheet).unwrap_err();
        assert!(err.to_string().contains("more than once"));
    }

    #[test]
    fn reload_replaces_local_from_sheet() {
        let (mut books, mut sheet) = seed_local(1001);
        books
            .conn()
            .execute(
                "UPDATE vouchers SET vendor = 'LOCAL' WHERE voucher_number = 1001",
                [],
            )
            .unwrap();
        mark_dirty(&books, 1001).unwrap();
        reload_voucher_with(&mut books, 1001, &mut sheet).unwrap();
        let local = load_local_voucher(&books, 1001).unwrap();
        assert_ne!(local.parsed.vendor, "LOCAL");
        assert!(!local.is_dirty);
    }

    #[test]
    fn hive_submit_writes_register_cells_not_raw() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[voucher_row(1001, 0.4)])).unwrap();
        let mut hive = crate::hive::MemoryHive::default();
        hive.ensure_tab(
            KIND_VOUCHER,
            &crate::hive_plan::report_headers(KIND_VOUCHER),
        )
        .unwrap();
        let out = submit_voucher_hive(&mut books, 1001, &mut hive).unwrap();
        assert_eq!(out, SubmitOutcome::Ok { voucher_number: 1001 });
        let row = hive.get_row(KIND_VOUCHER, "1001").unwrap().unwrap();
        assert_eq!(row.cells[0], "1001");
        assert!(hive.get_row("voucher_raw", "1001").unwrap().is_none());
        assert!(!is_dirty_flag(&books, 1001).unwrap());
    }

    #[test]
    fn raw_archive_sheet_refuses_writes() {
        let account = crate::sheets::parse_service_account(
            r#"{
              "client_email": "sa@example.com",
              "private_key": "-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----\n"
            }"#,
        )
        .unwrap();
        let mut sheet = GoogleRawArchive { account };
        let err = sheet.append_row(&["1001".into()]).unwrap_err();
        assert!(err.to_string().contains("read-only"));
        let err = sheet.update_row(2, &["1001".into()]).unwrap_err();
        assert!(err.to_string().contains("read-only"));
    }
}
