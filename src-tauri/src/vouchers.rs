//! Voucher_Raw_Data → SQLite. UI never calls this on paint.
//! Dirty local vouchers block Refresh. No silent overwrite.

use std::collections::{BTreeMap, HashSet};

use rusqlite::{params, Transaction};

use crate::core::business_rules::{
    amount_or_zero, empty_payment, parse_voucher_number, should_skip_sheet_row, summarize_voucher,
    PaymentBlock, VoucherInput, MAX_PAYMENT_BLOCKS,
};
use crate::db::LocalBooks;
use crate::online::is_online;
use crate::sheets::{
    create_tab_with_headers, credentials_exist, fetch_sheet_values, is_missing_tab_error,
    load_service_account,
};
use crate::{BooksError, Result, VOUCHER_RAW_TAB};

/// Column A + voucher_date + 7 register fields. Payment blocks start at index 9.
/// Real sheet layout (loopbook `voucher-data.ts`): A is integer voucher_number.
const HEADER_COLS: usize = 9;
const SLOT_WIDTH: usize = 11;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowError {
    pub voucher_number: Option<i64>,
    pub row: i32,
    pub error_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RefreshOutcome {
    Ok {
        imported: u32,
        skipped: u32,
        errors: Vec<RowError>,
        warning: Option<String>,
    },
    Dirty {
        voucher_numbers: Vec<i64>,
        keys: Vec<crate::hive::DirtyKey>,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherSummary {
    pub count: i64,
    pub last_synced: Option<String>,
    pub pending: i64,
    pub partial: i64,
    pub full: i64,
    pub advance: i64,
    pub missing_tax: i64,
    pub dirty: i64,
    pub remaining_total: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherListRow {
    pub voucher_number: i64,
    pub vendor: String,
    pub project: String,
    pub tax_invoice: String,
    pub total_value: f64,
    pub total_paid: f64,
    pub remaining: f64,
    pub status: String,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentView {
    pub slot: i32,
    pub pi_no: String,
    pub pi_date: String,
    pub pi_value: f64,
    pub paid: f64,
    pub remaining: f64,
    pub payment_date: String,
    pub remarks: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherView {
    pub voucher_number: i64,
    pub tax_invoice: String,
    pub vendor: String,
    pub project: String,
    pub gst: String,
    pub comments: String,
    pub total_value: f64,
    pub total_paid: f64,
    pub remaining: f64,
    pub status: String,
    pub is_dirty: bool,
    pub payments: Vec<PaymentView>,
}

#[derive(Debug, Clone)]
pub struct ParsedVoucher {
    pub voucher_number: i64,
    pub voucher_date: String,
    pub tax_invoice: String,
    pub vendor: String,
    pub bank: String,
    pub account_number: String,
    pub ifsc: String,
    pub gst: String,
    pub project: String,
    pub payments: Vec<PaymentBlock>,
}

#[derive(Debug, Clone)]
pub struct ParseReport {
    pub vouchers: Vec<ParsedVoucher>,
    pub errors: Vec<RowError>,
    pub skipped: u32,
}

fn cell(row: &[String], i: usize) -> &str {
    row.get(i).map(|s| s.as_str()).unwrap_or("")
}

fn is_header_row(cols: &[String]) -> bool {
    let head = cols
        .iter()
        .take(HEADER_COLS)
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    if head.contains("vendor name")
        || head.contains("tax inv")
        || head.contains("voucher date")
        || head.contains("voucher_number")
        || head.contains("serial")
    {
        return true;
    }
    let a = cell(cols, 0).trim().to_ascii_lowercase();
    a == "f" || a == "voucher_number" || a == "voucher number"
}

fn slot_occupied(slot: &PaymentBlock) -> bool {
    slot.pi_value_rupees.unwrap_or(0.0) != 0.0
        || slot.paid_rupees.unwrap_or(0.0) != 0.0
        || slot.tds_rupees.unwrap_or(0.0) != 0.0
        || !slot.payment_date.trim().is_empty()
        || !slot.payment_details.trim().is_empty()
        || !slot.pi_no.trim().is_empty()
}

fn slot_from(row: &[String], start: usize, slot: i32) -> PaymentBlock {
    let mut block = empty_payment(slot);
    block.pi_no = cell(row, start).trim().to_string();
    block.pi_date = cell(row, start + 1).trim().to_string();
    block.pi_value_rupees = Some(amount_or_zero(cell(row, start + 2)));
    block.payment_percent = Some(amount_or_zero(cell(row, start + 3)));
    block.description = cell(row, start + 4).trim().to_string();
    block.tds_rupees = Some(amount_or_zero(cell(row, start + 5)));
    block.paid_rupees = Some(amount_or_zero(cell(row, start + 6)));
    block.payment_details = cell(row, start + 7).trim().to_string();
    block.payment_date = cell(row, start + 8).trim().to_string();
    block.remaining_rupees = Some(amount_or_zero(cell(row, start + 9)));
    block.remarks = cell(row, start + 10).trim().to_string();
    block
}

fn log_skip(row: i32, voucher: Option<i64>, error_type: &str) {
    crate::log::event(
        crate::log::Level::Error,
        "refresh",
        "skip",
        voucher,
        error_type,
    );
    match voucher {
        Some(n) => eprintln!("t-books refresh skip row={row} voucher={n} type={error_type}"),
        None => eprintln!("t-books refresh skip row={row} voucher=none type={error_type}"),
    }
}

pub fn parse_voucher_values(values: &[Vec<String>]) -> ParseReport {
    let mut errors = Vec::new();
    let mut skipped = 0u32;
    let mut by_number: BTreeMap<i64, ParsedVoucher> = BTreeMap::new();

    for (i, cols) in values.iter().enumerate() {
        let row_no = (i + 1) as i32;
        if cols.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        if is_header_row(cols) {
            continue;
        }
        let raw_a = cell(cols, 0);
        let vendor = cell(cols, 3);
        if should_skip_sheet_row(raw_a, vendor) {
            skipped += 1;
            let error_type = if parse_voucher_number(raw_a).is_none() {
                "invalid_voucher_number"
            } else {
                "skipped_empty"
            };
            log_skip(row_no, parse_voucher_number(raw_a), error_type);
            errors.push(RowError {
                voucher_number: parse_voucher_number(raw_a),
                row: row_no,
                error_type: error_type.into(),
            });
            continue;
        }
        let Some(voucher_number) = parse_voucher_number(raw_a) else {
            skipped += 1;
            let error_type = "invalid_voucher_number";
            log_skip(row_no, None, error_type);
            errors.push(RowError {
                voucher_number: None,
                row: row_no,
                error_type: error_type.into(),
            });
            continue;
        };
        by_number.insert(voucher_number, parsed_from_cells(voucher_number, cols));
    }

    ParseReport {
        vouchers: by_number.into_values().collect(),
        errors,
        skipped,
    }
}

pub fn parse_one_voucher_row(cols: &[String]) -> Option<ParsedVoucher> {
    if cols.iter().all(|c| c.trim().is_empty()) || is_header_row(cols) {
        return None;
    }
    let voucher_number = parse_voucher_number(cell(cols, 0))?;
    Some(parsed_from_cells(voucher_number, cols))
}

fn parsed_from_cells(voucher_number: i64, cols: &[String]) -> ParsedVoucher {
    let mut payments = Vec::new();
    for s in 0..MAX_PAYMENT_BLOCKS {
        let start = HEADER_COLS + s * SLOT_WIDTH;
        let slot = slot_from(cols, start, (s as i32) + 1);
        if slot_occupied(&slot) {
            payments.push(slot);
        }
    }
    ParsedVoucher {
        voucher_number,
        voucher_date: cell(cols, 1).trim().to_string(),
        tax_invoice: cell(cols, 2).trim().to_string(),
        vendor: cell(cols, 3).trim().to_string(),
        bank: cell(cols, 4).trim().to_string(),
        account_number: cell(cols, 5).trim().to_string(),
        ifsc: cell(cols, 6).trim().to_string(),
        gst: cell(cols, 7).trim().to_string(),
        project: cell(cols, 8).trim().to_string(),
        payments,
    }
}

pub fn fingerprint_parsed(row: &ParsedVoucher) -> String {
    crate::core::business_rules::voucher_fingerprint(
        row.voucher_number,
        &row.voucher_date,
        &row.tax_invoice,
        &row.vendor,
        &row.bank,
        &row.account_number,
        &row.ifsc,
        &row.gst,
        &row.project,
        &row.payments,
    )
}

fn cell_num(value: Option<f64>) -> String {
    crate::core::business_rules::canon_num(value)
}

pub fn voucher_to_sheet_row(row: &ParsedVoucher) -> Vec<String> {
    let mut cells = vec![
        row.voucher_number.to_string(),
        row.voucher_date.trim().to_string(),
        row.tax_invoice.trim().to_string(),
        row.vendor.trim().to_string(),
        row.bank.trim().to_string(),
        row.account_number.trim().to_string(),
        row.ifsc.trim().to_string(),
        row.gst.trim().to_string(),
        row.project.trim().to_string(),
    ];
    let mut slots: Vec<PaymentBlock> = (1..=MAX_PAYMENT_BLOCKS as i32)
        .map(empty_payment)
        .collect();
    for block in row.payments.iter().take(MAX_PAYMENT_BLOCKS) {
        let idx = (block.slot.clamp(1, MAX_PAYMENT_BLOCKS as i32) as usize) - 1;
        slots[idx] = block.clone();
    }
    for block in slots {
        cells.push(block.pi_no.trim().to_string());
        cells.push(block.pi_date.trim().to_string());
        cells.push(cell_num(block.pi_value_rupees));
        cells.push(cell_num(block.payment_percent));
        cells.push(block.description.trim().to_string());
        cells.push(cell_num(block.tds_rupees));
        cells.push(cell_num(block.paid_rupees));
        cells.push(block.payment_details.trim().to_string());
        cells.push(block.payment_date.trim().to_string());
        cells.push(cell_num(block.remaining_rupees));
        cells.push(block.remarks.trim().to_string());
    }
    cells
}

pub fn list_dirty_vouchers(books: &LocalBooks) -> Result<Vec<i64>> {
    let mut stmt = books.conn().prepare(
        "SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1 ORDER BY voucher_number",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Dirty guard: if any local dirty row (any kind) is unsynced, Refresh must stop.
pub fn dirty_guard(books: &LocalBooks) -> Result<Option<Vec<i64>>> {
    let keys = crate::hive::list_dirty_keys(books)?;
    if keys.is_empty() {
        Ok(None)
    } else {
        let dirty: Vec<i64> = keys
            .iter()
            .filter(|k| k.kind == crate::hive::KIND_VOUCHER)
            .filter_map(|k| k.key.parse().ok())
            .collect();
        crate::log::event(
            crate::log::Level::Warn,
            "refresh",
            "dirty_guard",
            dirty.first().copied(),
            "blocked",
        );
        Ok(Some(dirty))
    }
}

fn now_sql(books: &LocalBooks) -> Result<String> {
    Ok(books
        .conn()
        .query_row("SELECT datetime('now')", [], |row| row.get(0))?)
}

fn delete_dirty_in_tx(tx: &Transaction<'_>) -> Result<u32> {
    tx.execute(
        "DELETE FROM voucher_payments WHERE voucher_number IN (SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1)",
        [],
    )?;
    let deleted = tx.execute(
        "DELETE FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1",
        [],
    )?;
    Ok(deleted as u32)
}

pub fn discard_dirty_vouchers(books: &mut LocalBooks) -> Result<u32> {
    let tx = books.conn_mut().transaction()?;
    let deleted = delete_dirty_in_tx(&tx)?;
    tx.commit()?;
    Ok(deleted)
}

fn as_input(row: &ParsedVoucher) -> VoucherInput {
    VoucherInput {
        voucher_number: row.voucher_number,
        tax_invoice: row.tax_invoice.clone(),
        vendor: row.vendor.clone(),
        bank: row.bank.clone(),
        account_number: row.account_number.clone(),
        ifsc: row.ifsc.clone(),
        gst: row.gst.clone(),
        project: row.project.clone(),
        comments: row
            .payments
            .iter()
            .map(|p| p.remarks.as_str())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        payments: row.payments.clone(),
    }
}

fn empty_warning(skipped: u32) -> String {
    if skipped > 0 {
        format!(
            "{VOUCHER_RAW_TAB} had no valid voucher numbers. Local vouchers were kept. {skipped} row(s) skipped."
        )
    } else {
        format!("{VOUCHER_RAW_TAB} is empty. Local vouchers were kept.")
    }
}

fn apply_parsed(
    books: &mut LocalBooks,
    parsed: &ParseReport,
    discard_dirty: bool,
) -> Result<RefreshOutcome> {
    let synced = now_sql(books)?;
    let tx = books.conn_mut().transaction()?;
    if discard_dirty {
        delete_dirty_in_tx(&tx)?;
    }

    if parsed.vouchers.is_empty() {
        tx.execute(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('voucher_last_synced', ?1)",
            params![synced],
        )?;
        tx.commit()?;
        return Ok(RefreshOutcome::Ok {
            imported: 0,
            skipped: parsed.skipped,
            errors: parsed.errors.clone(),
            warning: Some(empty_warning(parsed.skipped)),
        });
    }

    let dirty_set: HashSet<i64> = {
        let mut stmt = tx.prepare(
            "SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        let mut set = HashSet::new();
        for row in rows {
            set.insert(row?);
        }
        set
    };

    let mut imported = 0u32;
    {
        let mut ins_v = tx.prepare(
            r#"
            INSERT INTO vouchers (
              voucher_number, voucher_date, tax_invoice, vendor, bank, account_number, ifsc, gst, project,
              comments, description, dirty, is_dirty, source, source_hash, fingerprint, total_value, total_paid, remaining, status, tax_flag,
              created_at, updated_at
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
              ?10, ?11, 0, 0, 'sheet', ?12, ?12, ?13, ?14, ?15, ?16, ?17,
              datetime('now'), datetime('now')
            )
            ON CONFLICT(voucher_number) DO UPDATE SET
              voucher_date = excluded.voucher_date,
              tax_invoice = excluded.tax_invoice,
              vendor = excluded.vendor,
              bank = excluded.bank,
              account_number = excluded.account_number,
              ifsc = excluded.ifsc,
              gst = excluded.gst,
              project = excluded.project,
              comments = excluded.comments,
              description = excluded.description,
              dirty = 0,
              is_dirty = 0,
              source = 'sheet',
              source_hash = excluded.source_hash,
              fingerprint = excluded.fingerprint,
              total_value = excluded.total_value,
              total_paid = excluded.total_paid,
              remaining = excluded.remaining,
              status = excluded.status,
              tax_flag = excluded.tax_flag,
              updated_at = excluded.updated_at
            WHERE COALESCE(vouchers.is_dirty, vouchers.dirty, 0) = 0
            "#,
        )?;
        let mut del_pay = tx.prepare("DELETE FROM voucher_payments WHERE voucher_number = ?1")?;
        let mut ins_pay = tx.prepare(
            r#"
            INSERT INTO voucher_payments (
              voucher_number, slot, pi_no, pi_date, pi_value_rupees, payment_percent, description,
              tds_rupees, paid_rupees, payment_details, payment_date, remaining_rupees, remarks
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )?;
        let mut ins_vendor = tx.prepare(
            r#"
            INSERT INTO vendors (vendor, bank, account_number, ifsc, gst, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
            ON CONFLICT(vendor) DO UPDATE SET
              bank = excluded.bank,
              account_number = excluded.account_number,
              ifsc = excluded.ifsc,
              gst = excluded.gst,
              updated_at = excluded.updated_at
            "#,
        )?;
        let mut ins_project = tx.prepare(
            r#"
            INSERT INTO projects (project, updated_at)
            VALUES (?1, datetime('now'))
            ON CONFLICT(project) DO UPDATE SET updated_at = excluded.updated_at
            "#,
        )?;

        for row in &parsed.vouchers {
            if dirty_set.contains(&row.voucher_number) {
                continue;
            }
            let computed = summarize_voucher(&as_input(row));
            let description = row
                .payments
                .iter()
                .map(|p| p.description.as_str())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            let comments = row
                .payments
                .iter()
                .map(|p| p.remarks.as_str())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            let hash = fingerprint_parsed(row);
            ins_v.execute(params![
                row.voucher_number,
                row.voucher_date,
                row.tax_invoice,
                row.vendor,
                row.bank,
                row.account_number,
                row.ifsc,
                row.gst,
                row.project,
                comments,
                description,
                hash,
                computed.total_value,
                computed.total_paid,
                computed.remaining,
                computed.status,
                computed.tax_flag.as_str(),
            ])?;
            del_pay.execute(params![row.voucher_number])?;
            for p in &row.payments {
                ins_pay.execute(params![
                    row.voucher_number,
                    p.slot,
                    p.pi_no,
                    p.pi_date,
                    p.pi_value_rupees,
                    p.payment_percent,
                    p.description,
                    p.tds_rupees,
                    p.paid_rupees,
                    p.payment_details,
                    p.payment_date,
                    p.remaining_rupees,
                    p.remarks,
                ])?;
            }
            if !row.vendor.is_empty() {
                ins_vendor.execute(params![
                    row.vendor,
                    row.bank,
                    row.account_number,
                    row.ifsc,
                    row.gst,
                ])?;
            }
            if !row.project.is_empty() {
                ins_project.execute(params![row.project])?;
            }
            imported += 1;
        }
    }

    tx.execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('voucher_last_synced', ?1)",
        params![synced],
    )?;
    tx.commit()?;

    let warning = if parsed.skipped > 0 {
        Some(format!(
            "Data refreshed. {} row(s) skipped (invalid voucher number).",
            parsed.skipped
        ))
    } else {
        None
    };
    Ok(RefreshOutcome::Ok {
        imported,
        skipped: parsed.skipped,
        errors: parsed.errors.clone(),
        warning,
    })
}

pub fn apply_voucher_rows(books: &mut LocalBooks, parsed: &ParseReport) -> Result<RefreshOutcome> {
    apply_parsed(books, parsed, false)
}

pub fn apply_voucher_rows_discarding_dirty(
    books: &mut LocalBooks,
    parsed: &ParseReport,
) -> Result<RefreshOutcome> {
    apply_parsed(books, parsed, true)
}

fn require_online_and_credentials() -> Result<()> {
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot refresh vouchers.",
        ));
    }
    if !credentials_exist() {
        return Err(BooksError::from(
            "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
        ));
    }
    Ok(())
}

fn fetch_parsed_from_google(allow_bootstrap: bool) -> Result<ParseReport> {
    require_online_and_credentials()?;
    let account = load_service_account()?;
    match fetch_sheet_values(&account, VOUCHER_RAW_TAB) {
        Ok(values) => Ok(parse_voucher_values(&values)),
        Err(err) if allow_bootstrap && is_missing_tab_error(&err.to_string()) => {
            let headers: Vec<String> = crate::hive::tab_headers(crate::hive::KIND_VOUCHER);
            create_tab_with_headers(&account, VOUCHER_RAW_TAB, &headers)?;
            Ok(parse_voucher_values(&[headers]))
        }
        Err(err) => Err(err),
    }
}

pub fn refresh_vouchers(books: &mut LocalBooks) -> Result<RefreshOutcome> {
    refresh_vouchers_with(books, false)
}

pub fn refresh_vouchers_with(books: &mut LocalBooks, allow_bootstrap: bool) -> Result<RefreshOutcome> {
    require_online_and_credentials()?;
    if let Some(voucher_numbers) = dirty_guard(books)? {
        let keys = crate::hive::list_dirty_keys(books)?;
        return Ok(RefreshOutcome::Dirty {
            voucher_numbers,
            keys,
        });
    }
    let parsed = fetch_parsed_from_google(allow_bootstrap)?;
    apply_voucher_rows(books, &parsed)
}

pub fn force_refresh_vouchers(books: &mut LocalBooks) -> Result<RefreshOutcome> {
    require_online_and_credentials()?;
    let parsed = fetch_parsed_from_google(false)?;
    apply_voucher_rows_discarding_dirty(books, &parsed)
}

pub fn get_voucher_last_synced(books: &LocalBooks) -> Option<String> {
    books
        .conn()
        .query_row(
            "SELECT value FROM app_meta WHERE key = 'voucher_last_synced' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.is_empty())
}

pub fn get_voucher_summary(books: &LocalBooks) -> Result<VoucherSummary> {
    let count: i64 = books
        .conn()
        .query_row("SELECT COUNT(*) FROM vouchers", [], |row| row.get(0))?;
    let dirty: i64 = books.conn().query_row(
        "SELECT COUNT(*) FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1",
        [],
        |row| row.get(0),
    )?;
    let status_count = |needle: &str| -> Result<i64> {
        Ok(books.conn().query_row(
            "SELECT COUNT(*) FROM vouchers WHERE lower(COALESCE(status, '')) LIKE ?1",
            params![format!("%{needle}%")],
            |row| row.get(0),
        )?)
    };
    let remaining_total: f64 = books.conn().query_row(
        "SELECT COALESCE(SUM(remaining), 0) FROM vouchers",
        [],
        |row| row.get(0),
    )?;
    Ok(VoucherSummary {
        count,
        last_synced: get_voucher_last_synced(books),
        pending: status_count("pending")?,
        partial: status_count("partial")?,
        full: status_count("full")?,
        advance: status_count("advance")?,
        missing_tax: status_count("missing")?,
        dirty,
        remaining_total,
    })
}

pub fn list_vouchers(books: &LocalBooks) -> Result<Vec<VoucherListRow>> {
    let mut stmt = books.conn().prepare(
        "SELECT voucher_number, vendor, project, tax_invoice, COALESCE(total_value, 0),
                COALESCE(total_paid, 0), COALESCE(remaining, 0), COALESCE(status, ''),
                COALESCE(is_dirty, dirty, 0)
         FROM vouchers
         ORDER BY voucher_number DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(VoucherListRow {
            voucher_number: row.get(0)?,
            vendor: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            project: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            tax_invoice: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            total_value: row.get(4)?,
            total_paid: row.get(5)?,
            remaining: row.get(6)?,
            status: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
            is_dirty: row.get::<_, i64>(8)? != 0,
        })
    })?;
    let mut out = Vec::with_capacity(64);
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_voucher(books: &LocalBooks, voucher_number: i64) -> Result<VoucherView> {
    let conn = books.conn();
    let row = conn
        .query_row(
            "SELECT voucher_number, tax_invoice, vendor, project, gst, comments,
                    COALESCE(total_value, 0), COALESCE(total_paid, 0), COALESCE(remaining, 0),
                    COALESCE(status, ''), COALESCE(is_dirty, dirty, 0)
             FROM vouchers WHERE voucher_number = ?1",
            params![voucher_number],
            |row| {
                Ok(VoucherView {
                    voucher_number: row.get(0)?,
                    tax_invoice: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    vendor: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    project: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    gst: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    comments: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                    total_value: row.get(6)?,
                    total_paid: row.get(7)?,
                    remaining: row.get(8)?,
                    status: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                    is_dirty: row.get::<_, i64>(10)? != 0,
                    payments: Vec::new(),
                })
            },
        )
        .map_err(|_| BooksError::from(format!("Voucher {voucher_number} is not on this PC.")))?;

    let mut stmt = conn.prepare(
        "SELECT slot, pi_no, pi_date, COALESCE(pi_value_rupees, 0), COALESCE(paid_rupees, 0),
                COALESCE(remaining_rupees, 0), payment_date, remarks
         FROM voucher_payments WHERE voucher_number = ?1 ORDER BY slot LIMIT 5",
    )?;
    let payments = stmt
        .query_map(params![voucher_number], |p| {
            Ok(PaymentView {
                slot: p.get(0)?,
                pi_no: p.get::<_, Option<String>>(1)?.unwrap_or_default(),
                pi_date: p.get::<_, Option<String>>(2)?.unwrap_or_default(),
                pi_value: p.get(3)?,
                paid: p.get(4)?,
                remaining: p.get(5)?,
                payment_date: p.get::<_, Option<String>>(6)?.unwrap_or_default(),
                remarks: p.get::<_, Option<String>>(7)?.unwrap_or_default(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let payments: Vec<PaymentView> = payments
        .into_iter()
        .filter(|p| {
            p.pi_value != 0.0
                || p.paid != 0.0
                || !p.pi_no.trim().is_empty()
                || !p.payment_date.trim().is_empty()
        })
        .collect();

    Ok(VoucherView {
        payments,
        ..row
    })
}

pub fn mark_dirty(books: &LocalBooks, voucher_number: i64) -> Result<()> {
    books.conn().execute(
        "UPDATE vouchers SET is_dirty = 1, dirty = 1, updated_at = datetime('now') WHERE voucher_number = ?1",
        params![voucher_number],
    )?;
    Ok(())
}

pub struct LocalVoucher {
    pub parsed: ParsedVoucher,
    pub source_hash: String,
    #[allow(dead_code)]
    pub is_dirty: bool,
}

pub fn load_local_voucher(books: &LocalBooks, voucher_number: i64) -> Result<LocalVoucher> {
    let mut stmt = books.conn().prepare(
        "SELECT voucher_number, COALESCE(voucher_date,''), COALESCE(tax_invoice,''), COALESCE(vendor,''),
                COALESCE(bank,''), COALESCE(account_number,''), COALESCE(ifsc,''), COALESCE(gst,''),
                COALESCE(project,''), COALESCE(source_hash, fingerprint, ''), COALESCE(is_dirty, dirty, 0)
         FROM vouchers WHERE voucher_number = ?1",
    )?;
    let parsed_row = stmt
        .query_row(params![voucher_number], |row| {
            Ok((
                ParsedVoucher {
                    voucher_number: row.get(0)?,
                    voucher_date: row.get(1)?,
                    tax_invoice: row.get(2)?,
                    vendor: row.get(3)?,
                    bank: row.get(4)?,
                    account_number: row.get(5)?,
                    ifsc: row.get(6)?,
                    gst: row.get(7)?,
                    project: row.get(8)?,
                    payments: Vec::new(),
                },
                row.get::<_, String>(9)?,
                row.get::<_, i64>(10)? != 0,
            ))
        })
        .map_err(|_| BooksError::from(format!("Voucher {voucher_number} is not on this PC.")))?;
    let (mut parsed, source_hash, is_dirty) = parsed_row;
    let mut pay = books.conn().prepare(
        "SELECT slot, COALESCE(pi_no,''), COALESCE(pi_date,''), pi_value_rupees, payment_percent,
                COALESCE(description,''), tds_rupees, paid_rupees, COALESCE(payment_details,''),
                COALESCE(payment_date,''), remaining_rupees, COALESCE(remarks,'')
         FROM voucher_payments WHERE voucher_number = ?1 ORDER BY slot LIMIT 5",
    )?;
    let rows = pay.query_map(params![voucher_number], |row| {
        Ok(PaymentBlock {
            slot: row.get(0)?,
            pi_no: row.get(1)?,
            pi_date: row.get(2)?,
            pi_value_rupees: row.get(3)?,
            payment_percent: row.get(4)?,
            description: row.get(5)?,
            tds_rupees: row.get(6)?,
            paid_rupees: row.get(7)?,
            payment_details: row.get(8)?,
            payment_date: row.get(9)?,
            remaining_rupees: row.get(10)?,
            remarks: row.get(11)?,
        })
    })?;
    for row in rows {
        parsed.payments.push(row?);
    }
    Ok(LocalVoucher {
        parsed,
        source_hash,
        is_dirty,
    })
}

pub fn apply_one_force(books: &mut LocalBooks, row: &ParsedVoucher) -> Result<()> {
    let report = ParseReport {
        vouchers: vec![row.clone()],
        errors: Vec::new(),
        skipped: 0,
    };
    let tx = books.conn_mut().transaction()?;
    tx.execute(
        "UPDATE vouchers SET is_dirty = 0, dirty = 0 WHERE voucher_number = ?1",
        params![row.voucher_number],
    )?;
    tx.execute(
        "DELETE FROM pending_submit WHERE kind = 'voucher' AND key = ?1",
        params![row.voucher_number.to_string()],
    )?;
    tx.commit()?;
    apply_parsed(books, &report, false)?;
    Ok(())
}

pub fn mark_submitted(books: &LocalBooks, voucher_number: i64, source_hash: &str) -> Result<()> {
    let n = books.conn().execute(
        "UPDATE vouchers SET is_dirty = 0, dirty = 0, source_hash = ?1, fingerprint = ?1,
         source = 'sheet', hive_rev = COALESCE(hive_rev, 0) + 1, updated_at = datetime('now')
         WHERE voucher_number = ?2",
        params![source_hash, voucher_number],
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
    use crate::open_memory;

    fn row_20() -> Vec<String> {
        let mut row = vec![
            "20".into(),
            "2026-01-10".into(),
            "INV-20".into(),
            "VEND_02".into(),
            "BANK_01".into(),
            "00000000".into(),
            "IFSC0000001".into(),
            "GST00DUMMY".into(),
            "CUST_01".into(),
        ];
        row.extend([
            "PI-A".into(),
            "2026-01-11".into(),
            "10000".into(),
            "40".into(),
            "First".into(),
            "0".into(),
            "4000".into(),
            "UTR1".into(),
            "2026-01-12".into(),
            "6000".into(),
            "".into(),
        ]);
        row
    }

    #[test]
    fn column_a_must_be_integer() {
        let parsed = parse_voucher_values(&[
            vec!["voucher_number".into(), "date".into(), "tax inv".into()],
            row_20(),
            {
                let mut bad = row_20();
                bad[0] = "20.1".into();
                bad
            },
            vec!["VOUCHER_1001".into(), "".into(), "".into(), "VEND_02".into()],
        ]);
        assert_eq!(parsed.vouchers.len(), 1);
        assert_eq!(parsed.vouchers[0].voucher_number, 20);
        assert_eq!(parsed.vouchers[0].tax_invoice, "INV-20");
        assert_eq!(parsed.vouchers[0].vendor, "VEND_02");
        assert_eq!(parsed.vouchers[0].payments[0].pi_no, "PI-A");
        assert_eq!(parsed.skipped, 2);
        assert!(parsed
            .errors
            .iter()
            .all(|e| e.error_type == "invalid_voucher_number"));
        assert!(parsed.errors.iter().all(|e| e.voucher_number.is_none()));
    }

    #[test]
    fn blank_numeric_is_zero_and_never_panics() {
        let mut row = row_20();
        row[9 + 2] = "not-a-number".into();
        row[9 + 5] = "".into();
        let parsed = parse_voucher_values(&[row]);
        assert_eq!(parsed.vouchers.len(), 1);
        assert_eq!(parsed.vouchers[0].payments[0].pi_value_rupees, Some(0.0));
        assert_eq!(parsed.vouchers[0].payments[0].tds_rupees, Some(0.0));
    }

    #[test]
    fn caps_at_five_payment_blocks() {
        let mut row = row_20();
        for s in 1..7 {
            row.extend([
                format!("PI-{s}"),
                "2026-01-11".into(),
                "1".into(),
                "0".into(),
                "".into(),
                "0".into(),
                "0".into(),
                "".into(),
                "".into(),
                "0".into(),
                "".into(),
            ]);
        }
        let parsed = parse_voucher_values(&[row]);
        assert_eq!(parsed.vouchers[0].payments.len(), 5);
    }

    #[test]
    fn dirty_guard_stops_without_wiping() {
        let mut books = open_memory().unwrap();
        let parsed = parse_voucher_values(&[row_20()]);
        apply_voucher_rows(&mut books, &parsed).unwrap();
        mark_dirty(&books, 20).unwrap();
        let dirty = dirty_guard(&books).unwrap().unwrap();
        assert_eq!(dirty, vec![20]);
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn apply_overwrites_clean_and_keeps_status() {
        let mut books = open_memory().unwrap();
        let parsed = parse_voucher_values(&[row_20()]);
        let out = apply_voucher_rows(&mut books, &parsed).unwrap();
        match out {
            RefreshOutcome::Ok {
                imported, skipped, ..
            } => {
                assert_eq!(imported, 1);
                assert_eq!(skipped, 0);
            }
            RefreshOutcome::Dirty { .. } => panic!("not dirty"),
        }
        let status: String = books
            .conn()
            .query_row(
                "SELECT status FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "Partial Payment");
        let remaining: f64 = books
            .conn()
            .query_row(
                "SELECT remaining FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 6000.0);
        let total_value: f64 = books
            .conn()
            .query_row(
                "SELECT total_value FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total_value, 10000.0);
        let total_paid: f64 = books
            .conn()
            .query_row(
                "SELECT total_paid FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total_paid, 4000.0);
    }

    #[test]
    fn empty_sheet_keeps_local() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20()])).unwrap();
        let out = apply_voucher_rows(&mut books, &parse_voucher_values(&[])).unwrap();
        match out {
            RefreshOutcome::Ok {
                imported, warning, ..
            } => {
                assert_eq!(imported, 0);
                assert!(warning.unwrap().contains("empty"));
            }
            RefreshOutcome::Dirty { .. } => panic!("not dirty"),
        }
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn discard_deletes_only_dirty() {
        let mut books = open_memory().unwrap();
        let mut second = row_20();
        second[0] = "21".into();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20(), second])).unwrap();
        mark_dirty(&books, 20).unwrap();
        discard_dirty_vouchers(&mut books).unwrap();
        let left: Vec<i64> = {
            let mut stmt = books
                .conn()
                .prepare("SELECT voucher_number FROM vouchers ORDER BY 1")
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .map(|v| v.unwrap())
                .collect()
        };
        assert_eq!(left, vec![21]);
    }

    #[test]
    fn refresh_does_not_touch_inventory() {
        let mut books = open_memory().unwrap();
        books
            .conn()
            .execute_batch(
                "INSERT INTO inventory (item_name, type, size, quantity, cost, project)
                   VALUES ('kept', 'cable', '10mm', 1, 0, '');
                 INSERT INTO documents (name, path, linked_type, linked_id, created_at)
                   VALUES ('doc-1', '', 'project', 'CUST_01', datetime('now'));
                 CREATE TABLE IF NOT EXISTS po_master (id TEXT PRIMARY KEY);
                 INSERT INTO po_master (id) VALUES ('po-1');
                 INSERT INTO hr_people (name, role, salary) VALUES ('kept', '', 0);
                 INSERT INTO hr_payroll (person_id, month, pf_employee, pf_company, tds, total_paid)
                   VALUES (1, '2026-01', 0, 0, 0, 0);
                 INSERT INTO logistics (project, vehicle_number, invoice_number, start_date, reach_date)
                   VALUES ('', 'kept', '', '', '');",
            )
            .unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20()])).unwrap();
        let inv: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM inventory", [], |r| r.get(0))
            .unwrap();
        let docs: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
            .unwrap();
        let pos: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM po_master", [], |r| r.get(0))
            .unwrap();
        let hr: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM hr_payroll", [], |r| r.get(0))
            .unwrap();
        let log: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM logistics", [], |r| r.get(0))
            .unwrap();
        assert_eq!(inv, 1);
        assert_eq!(docs, 1);
        assert_eq!(pos, 1);
        assert_eq!(hr, 1);
        assert_eq!(log, 1);
    }

    #[test]
    fn dirty_row_is_not_overwritten() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20()])).unwrap();
        mark_dirty(&books, 20).unwrap();
        books
            .conn()
            .execute(
                "UPDATE vouchers SET vendor = 'LOCAL_KEEP' WHERE voucher_number = 20",
                [],
            )
            .unwrap();
        let mut incoming = row_20();
        incoming[3] = "SHEET_VENDOR".into();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[incoming])).unwrap();
        let vendor: String = books
            .conn()
            .query_row(
                "SELECT vendor FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vendor, "LOCAL_KEEP");
        let dirty: i64 = books
            .conn()
            .query_row(
                "SELECT is_dirty FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dirty, 1);
    }

    #[test]
    fn upsert_does_not_wipe_other_vouchers() {
        let mut books = open_memory().unwrap();
        let mut second = row_20();
        second[0] = "21".into();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20(), second])).unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20()])).unwrap();
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn discard_and_apply_replaces_dirty_from_sheet() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20()])).unwrap();
        mark_dirty(&books, 20).unwrap();
        books
            .conn()
            .execute(
                "UPDATE vouchers SET vendor = 'LOCAL_KEEP' WHERE voucher_number = 20",
                [],
            )
            .unwrap();
        let mut incoming = row_20();
        incoming[3] = "SHEET_VENDOR".into();
        apply_voucher_rows_discarding_dirty(&mut books, &parse_voucher_values(&[incoming])).unwrap();
        let vendor: String = books
            .conn()
            .query_row(
                "SELECT vendor FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vendor, "SHEET_VENDOR");
        let dirty: i64 = books
            .conn()
            .query_row(
                "SELECT is_dirty FROM vouchers WHERE voucher_number = 20",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dirty, 0);
    }

    #[test]
    fn thousand_rows_parse_without_panic() {
        let mut rows = Vec::with_capacity(1001);
        rows.push(vec!["voucher_number".into(), "date".into(), "tax inv".into()]);
        for i in 1..=1000 {
            let mut row = row_20();
            row[0] = i.to_string();
            rows.push(row);
        }
        let parsed = parse_voucher_values(&rows);
        assert_eq!(parsed.vouchers.len(), 1000);
        let mut books = open_memory().unwrap();
        let out = apply_voucher_rows(&mut books, &parsed).unwrap();
        match out {
            RefreshOutcome::Ok { imported, .. } => assert_eq!(imported, 1000),
            RefreshOutcome::Dirty { .. } => panic!("not dirty"),
        }
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1000);
        let pays: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM voucher_payments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(pays, 1000);
    }

    #[test]
    fn list_and_open_show_value_paid_remaining() {
        let mut books = open_memory().unwrap();
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row_20()])).unwrap();
        let list = list_vouchers(&books).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].voucher_number, 20);
        assert_eq!(list[0].remaining, 6000.0);
        let summary = get_voucher_summary(&books).unwrap();
        assert_eq!(summary.count, 1);
        assert_eq!(summary.remaining_total, 6000.0);
        let open = get_voucher(&books, 20).unwrap();
        assert_eq!(open.total_value, 10000.0);
        assert_eq!(open.total_paid, 4000.0);
        assert_eq!(open.remaining, 6000.0);
        assert_eq!(open.payments.len(), 1);
        assert!(get_voucher(&books, 99).is_err());
    }
}
