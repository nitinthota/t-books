//! Submit office rows to hive tabs when those tabs exist.
//! Missing tab → keep local; owner may bootstrap headers only. Never wipe.

use crate::core::business_rules::sha256_hex;
use crate::db::LocalBooks;
use crate::hive::{
    hive_conflict_message, sales_po_hive_key, submit_hive_row, tab_headers, tab_name, CasOutcome,
    EnsureTab, Hive, HiveRow, KIND_DOCUMENT, KIND_INVENTORY, KIND_LOGISTICS,
    KIND_PAYMENT, KIND_PURCHASE, KIND_SALARY, KIND_SALES_PO,
};
use crate::online::is_online;
use crate::hive_plan::{report_headers, status_targets, target_for_kind};
use crate::sheets::{
    append_row_on, create_tab_with_headers_on, credentials_exist, fetch_values_on,
    is_missing_tab_error, load_service_account, update_row_on, ServiceAccount,
};
use crate::{BooksError, Result};

pub fn missing_tab_message(kind: &str) -> String {
    format!(
        "{} tab is not on the sheet. Local data was not deleted.",
        tab_name(kind)
    )
}

fn fingerprint_cells(cells: &[String]) -> String {
    sha256_hex(&cells.join("\n"))
}

/// Load one office row as hive cells (without fp/rev).
pub fn office_cells(books: &LocalBooks, kind: &str, key: &str) -> Result<(Vec<String>, String, i64)> {
    match kind {
        KIND_PURCHASE => load_purchase(books, key),
        KIND_PAYMENT => load_payment(books, key),
        KIND_SALARY => load_salary(books, key),
        KIND_SALES_PO => load_sales_po(books, key),
        KIND_INVENTORY => load_inventory(books, key),
        KIND_LOGISTICS => load_logistics(books, key),
        KIND_DOCUMENT => load_document(books, key),
        _ => Err(BooksError::from("That hive kind cannot be submitted.")),
    }
}

fn load_purchase(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    books
        .conn()
        .query_row(
            "SELECT po_number, project, vendor, type, goods_received, tax_invoice_no, tax_invoice_date,
                    total_value, COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM purchase_po WHERE po_number = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![key],
            |row| {
                let cells = vec![
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    if row.get::<_, i64>(4)? != 0 { "Yes" } else { "No" }.into(),
                    row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    row.get::<_, Option<f64>>(7)?.unwrap_or(0.0).to_string(),
                ];
                let fp: String = row.get(8)?;
                let rev: i64 = row.get(9)?;
                Ok((cells, fp, rev))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn load_payment(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    books
        .conn()
        .query_row(
            "SELECT pay_number, po_number, vendor, amount_rupees, alloc_method, pay_class, pay_date,
                    COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM purchase_payments WHERE pay_number = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![key],
            |row| {
                let cells = vec![
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<f64>>(3)?.unwrap_or(0.0).to_string(),
                    row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                ];
                Ok((cells, row.get(7)?, row.get(8)?))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn load_salary(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    books
        .conn()
        .query_row(
            "SELECT p.salary_number, h.name, p.month, p.pay_kind, p.net_rupees,
                    COALESCE(p.source_hash,''), COALESCE(p.hive_rev,0)
             FROM hr_payroll p JOIN hr_people h ON h.id = p.person_id
             WHERE p.salary_number = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![key],
            |row| {
                let cells = vec![
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<f64>>(4)?.unwrap_or(0.0).to_string(),
                ];
                Ok((cells, row.get(5)?, row.get(6)?))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn load_sales_po(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let (po_number, project) = key.split_once('@').ok_or_else(|| {
        BooksError::from("Sales PO hive key must look like PO-1@CUST_01.")
    })?;
    books
        .conn()
        .query_row(
            "SELECT po_number, project, client, gst, total_value,
                    COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM sales_po WHERE po_number = ?1 COLLATE NOCASE AND project = ?2 COLLATE NOCASE LIMIT 1",
            rusqlite::params![po_number.trim(), project.trim()],
            |row| {
                let po: String = row.get(0)?;
                let proj: String = row.get(1)?;
                let cells = vec![
                    sales_po_hive_key(&po, &proj),
                    po,
                    proj,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<f64>>(4)?.unwrap_or(0.0).to_string(),
                ];
                Ok((cells, row.get(5)?, row.get(6)?))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn load_inventory(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let id: i64 = key
        .strip_prefix("INV-")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| BooksError::from("Inventory key must look like INV-1."))?;
    books
        .conn()
        .query_row(
            "SELECT id, item_name, type, size, quantity, cost, project,
                    COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM inventory WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let cells = vec![
                    format!("INV-{}", row.get::<_, i64>(0)?),
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<f64>>(4)?.unwrap_or(0.0).to_string(),
                    row.get::<_, Option<f64>>(5)?.unwrap_or(0.0).to_string(),
                    row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                ];
                Ok((cells, row.get(7)?, row.get(8)?))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn load_logistics(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let id: i64 = key
        .strip_prefix("TRIP-")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| BooksError::from("Logistics key must look like TRIP-1."))?;
    books
        .conn()
        .query_row(
            "SELECT id, project, vehicle_number, invoice_number, start_date, reach_date,
                    COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM logistics WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let cells = vec![
                    format!("TRIP-{}", row.get::<_, i64>(0)?),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                ];
                Ok((cells, row.get(6)?, row.get(7)?))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn load_document(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let id: i64 = key
        .strip_prefix("DOC-")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| BooksError::from("Document key must look like DOC-1."))?;
    books
        .conn()
        .query_row(
            "SELECT id, name, linked_type, linked_id, COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM documents WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let cells = vec![
                    format!("DOC-{}", row.get::<_, i64>(0)?),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                ];
                Ok((cells, row.get(4)?, row.get(5)?))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))
}

fn mark_office_submitted(books: &LocalBooks, kind: &str, key: &str, fp: &str, rev: i64) -> Result<()> {
    let n = match kind {
        KIND_PURCHASE => books.conn().execute(
            "UPDATE purchase_po SET is_dirty = 0, source_hash = ?1, hive_rev = ?2, updated_at = datetime('now')
             WHERE po_number = ?3 COLLATE NOCASE",
            rusqlite::params![fp, rev, key],
        )?,
        KIND_PAYMENT => books.conn().execute(
            "UPDATE purchase_payments SET is_dirty = 0, source_hash = ?1, hive_rev = ?2, updated_at = datetime('now')
             WHERE pay_number = ?3 COLLATE NOCASE",
            rusqlite::params![fp, rev, key],
        )?,
        KIND_SALARY => books.conn().execute(
            "UPDATE hr_payroll SET is_dirty = 0, source_hash = ?1, hive_rev = ?2
             WHERE salary_number = ?3 COLLATE NOCASE",
            rusqlite::params![fp, rev, key],
        )?,
        KIND_SALES_PO => {
            let (po_number, project) = key.split_once('@').unwrap_or(("", ""));
            books.conn().execute(
                "UPDATE sales_po SET is_dirty = 0, source_hash = ?1, hive_rev = ?2, updated_at = datetime('now')
                 WHERE po_number = ?3 COLLATE NOCASE AND project = ?4 COLLATE NOCASE",
                rusqlite::params![fp, rev, po_number.trim(), project.trim()],
            )?
        }
        KIND_INVENTORY => {
            let id: i64 = key.strip_prefix("INV-").and_then(|s| s.parse().ok()).unwrap_or(0);
            books.conn().execute(
                "UPDATE inventory SET is_dirty = 0, source_hash = ?1, hive_rev = ?2 WHERE id = ?3",
                rusqlite::params![fp, rev, id],
            )?
        }
        KIND_LOGISTICS => {
            let id: i64 = key.strip_prefix("TRIP-").and_then(|s| s.parse().ok()).unwrap_or(0);
            books.conn().execute(
                "UPDATE logistics SET is_dirty = 0, source_hash = ?1, hive_rev = ?2 WHERE id = ?3",
                rusqlite::params![fp, rev, id],
            )?
        }
        KIND_DOCUMENT => {
            let id: i64 = key.strip_prefix("DOC-").and_then(|s| s.parse().ok()).unwrap_or(0);
            books.conn().execute(
                "UPDATE documents SET is_dirty = 0, source_hash = ?1, hive_rev = ?2 WHERE id = ?3",
                rusqlite::params![fp, rev, id],
            )?
        }
        _ => 0,
    };
    if n == 0 {
        return Err(BooksError::from(format!("{key} is not on this PC.")));
    }
    Ok(())
}

pub fn submit_office_with(
    books: &LocalBooks,
    hive: &mut dyn Hive,
    kind: &str,
    key: &str,
    allow_bootstrap: bool,
) -> Result<CasOutcome> {
    let (cells, base_fp, base_rev) = office_cells(books, kind, key)?;
    let new_fp = fingerprint_cells(&cells);
    match hive.ensure_tab(kind, &report_headers(kind)) {
        Ok(EnsureTab::Exists) => {}
        Ok(EnsureTab::BootstrappedHeaders) => {
            if !allow_bootstrap {
                return Err(BooksError::from(missing_tab_message(kind)));
            }
            return Err(BooksError::from(format!(
                "{} tab was created with headers only. Submit again.",
                tab_name(kind)
            )));
        }
        Err(err) => return Err(err),
    }
    let payload = crate::hive::pending_payload_json(kind, key);
    let out = submit_hive_row(
        books, hive, kind, key, &base_fp, base_rev, &cells, &new_fp, &payload,
    )?;
    if let CasOutcome::Ok { ref fp, rev, .. } = out {
        mark_office_submitted(books, kind, key, fp, rev)?;
    }
    Ok(out)
}

pub struct GoogleOfficeHive {
    account: ServiceAccount,
    fail_if_missing: bool,
}

impl GoogleOfficeHive {
    pub fn connect() -> Result<Self> {
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
        Ok(Self {
            account: load_service_account()?,
            fail_if_missing: true,
        })
    }

    pub fn connect_bootstrap() -> Result<Self> {
        let mut hive = Self::connect()?;
        hive.fail_if_missing = false;
        Ok(hive)
    }

    fn probe(&self, kind: &str) -> HiveTabStatus {
        let target = match target_for_kind(kind) {
            Ok(t) => t,
            Err(err) => {
                return HiveTabStatus {
                    kind: kind.to_string(),
                    tab: tab_name(kind).to_string(),
                    present: false,
                    row_count: 0,
                    writable: false,
                    error: Some(err.to_string()),
                };
            }
        };
        let label = format!("{} / {}", target.workbook_title, target.tab);
        match fetch_values_on(&self.account, &target.spreadsheet_id, &target.tab) {
            Ok(values) => HiveTabStatus {
                kind: kind.to_string(),
                tab: label,
                present: true,
                row_count: parse_tab_rows(&values, kind).len() as i64,
                writable: target.writable,
                error: None,
            },
            Err(err) if is_missing_tab_error(&err.to_string()) => HiveTabStatus {
                kind: kind.to_string(),
                tab: label,
                present: false,
                row_count: 0,
                writable: target.writable,
                error: None,
            },
            Err(err) => HiveTabStatus {
                kind: kind.to_string(),
                tab: label,
                present: false,
                row_count: 0,
                writable: target.writable,
                error: Some(err.to_string()),
            },
        }
    }
}

fn parse_tab_rows(values: &[Vec<String>], kind: &str) -> Vec<HiveRow> {
    let mut out = Vec::new();
    for (i, row) in values.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let key = row.first().map(|s| s.trim().to_string()).unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        let fp = row.get(row.len().saturating_sub(2)).cloned().unwrap_or_default();
        let rev = row
            .last()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        out.push(HiveRow {
            key,
            kind: kind.to_string(),
            fp,
            rev,
            cells: row.clone(),
        });
    }
    out
}

impl Hive for GoogleOfficeHive {
    fn get_row(&self, kind: &str, key: &str) -> Result<Option<HiveRow>> {
        let target = target_for_kind(kind)?;
        let values = match fetch_values_on(&self.account, &target.spreadsheet_id, &target.tab) {
            Ok(v) => v,
            Err(err) if is_missing_tab_error(&err.to_string()) => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(parse_tab_rows(&values, kind)
            .into_iter()
            .find(|r| r.key.eq_ignore_ascii_case(key)))
    }

    fn cas_row(
        &mut self,
        kind: &str,
        key: &str,
        base_fp: &str,
        base_rev: i64,
        cells: &[String],
        new_fp: &str,
    ) -> Result<CasOutcome> {
        let target = target_for_kind(kind)?;
        if !target.writable {
            return Err(BooksError::from(
                "Voucher_Raw_Data is read-only. Write the structured hive instead. The original register was not changed.",
            ));
        }
        let values = match fetch_values_on(&self.account, &target.spreadsheet_id, &target.tab) {
            Ok(v) => v,
            Err(err) if is_missing_tab_error(&err.to_string()) => {
                return Err(BooksError::from(missing_tab_message(kind)));
            }
            Err(err) => return Err(err),
        };
        let mut body = cells.to_vec();
        body.push(new_fp.to_string());
        let existing: Vec<(u32, HiveRow)> = parse_tab_rows(&values, kind)
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.key.eq_ignore_ascii_case(key))
            .map(|(i, r)| ((i as u32) + 2, r))
            .collect();
        if existing.len() > 1 {
            return Err(BooksError::from(format!(
                "{key} appears more than once in the sheet."
            )));
        }
        if existing.is_empty() {
            if base_rev != 0 && !base_fp.is_empty() {
                return Ok(CasOutcome::Conflict {
                    key: key.to_string(),
                    message: hive_conflict_message(key),
                });
            }
            let mut row = body;
            row.push("1".into());
            append_row_on(&self.account, &target.spreadsheet_id, &target.tab, &row)?;
            return Ok(CasOutcome::Ok {
                key: key.to_string(),
                fp: new_fp.to_string(),
                rev: 1,
            });
        }
        let (sheet_row, row) = &existing[0];
        if row.fp == new_fp {
            return Ok(CasOutcome::Ok {
                key: key.to_string(),
                fp: row.fp.clone(),
                rev: row.rev,
            });
        }
        if row.fp != base_fp || row.rev != base_rev {
            return Ok(CasOutcome::Conflict {
                key: key.to_string(),
                message: hive_conflict_message(key),
            });
        }
        let next_rev = base_rev + 1;
        let mut written = body;
        written.push(next_rev.to_string());
        let last = format!("A{sheet_row}:Z{sheet_row}");
        update_row_on(
            &self.account,
            &target.spreadsheet_id,
            &target.tab,
            &last,
            &written,
        )?;
        Ok(CasOutcome::Ok {
            key: key.to_string(),
            fp: new_fp.to_string(),
            rev: next_rev,
        })
    }

    fn list_keys(&self, kind: &str) -> Result<Vec<String>> {
        let target = target_for_kind(kind)?;
        match fetch_values_on(&self.account, &target.spreadsheet_id, &target.tab) {
            Ok(values) => Ok(parse_tab_rows(&values, kind).into_iter().map(|r| r.key).collect()),
            Err(err) if is_missing_tab_error(&err.to_string()) => Ok(Vec::new()),
            Err(err) => Err(err),
        }
    }

    fn ensure_tab(&mut self, kind: &str, headers: &[String]) -> Result<EnsureTab> {
        let target = target_for_kind(kind)?;
        if !target.writable {
            return Err(BooksError::from(
                "Voucher_Raw_Data is read-only. Owner cannot bootstrap the archive tab.",
            ));
        }
        match fetch_values_on(&self.account, &target.spreadsheet_id, &target.tab) {
            Ok(_) => Ok(EnsureTab::Exists),
            Err(err) if is_missing_tab_error(&err.to_string()) => {
                if self.fail_if_missing {
                    return Err(BooksError::from(missing_tab_message(kind)));
                }
                let heads = if headers.is_empty() {
                    report_headers(kind)
                } else {
                    headers.to_vec()
                };
                create_tab_with_headers_on(
                    &self.account,
                    &target.spreadsheet_id,
                    &target.tab,
                    &heads,
                )?;
                Ok(EnsureTab::BootstrappedHeaders)
            }
            Err(err) => Err(err),
        }
    }
}

pub fn submit_office(
    books: &LocalBooks,
    kind: &str,
    key: &str,
    allow_bootstrap: bool,
) -> Result<CasOutcome> {
    let mut hive = GoogleOfficeHive::connect()?;
    hive.fail_if_missing = !allow_bootstrap;
    if allow_bootstrap {
        match hive.ensure_tab(kind, &report_headers(kind)) {
            Ok(EnsureTab::BootstrappedHeaders) => {
                return Err(BooksError::from(format!(
                    "{} tab was created with headers only. Submit again.",
                    tab_name(kind)
                )));
            }
            Ok(EnsureTab::Exists) => {}
            Err(err) => return Err(err),
        }
    }
    submit_office_with(books, &mut hive, kind, key, allow_bootstrap)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiveTabStatus {
    pub kind: String,
    pub tab: String,
    pub present: bool,
    pub row_count: i64,
    pub writable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiveStatus {
    pub online: bool,
    pub credentials_found: bool,
    pub tabs: Vec<HiveTabStatus>,
}

fn plan_tab_placeholders(error: Option<String>) -> Vec<HiveTabStatus> {
    match status_targets() {
        Ok(targets) => targets
            .into_iter()
            .map(|t| HiveTabStatus {
                kind: t.kind,
                tab: format!("{} / {}", t.workbook_title, t.tab),
                present: false,
                row_count: 0,
                writable: t.writable,
                error: error.clone(),
            })
            .collect(),
        Err(err) => vec![HiveTabStatus {
            kind: "hive".into(),
            tab: "Hive".into(),
            present: false,
            row_count: 0,
            writable: false,
            error: Some(err.to_string()),
        }],
    }
}

pub fn hive_status() -> HiveStatus {
    let online = is_online();
    let credentials_found = credentials_exist();
    if !online {
        return HiveStatus {
            online,
            credentials_found,
            tabs: plan_tab_placeholders(Some(
                "This PC is offline. Cannot inspect hive tabs.".into(),
            )),
        };
    }
    if !credentials_found {
        return HiveStatus {
            online,
            credentials_found,
            tabs: plan_tab_placeholders(Some(
                "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.".into(),
            )),
        };
    }
    let hive = match GoogleOfficeHive::connect() {
        Ok(h) => h,
        Err(err) => {
            return HiveStatus {
                online,
                credentials_found,
                tabs: plan_tab_placeholders(Some(err.to_string())),
            };
        }
    };
    let tabs = match status_targets() {
        Ok(targets) => targets.into_iter().map(|t| hive.probe(&t.kind)).collect(),
        Err(err) => plan_tab_placeholders(Some(err.to_string())),
    };
    HiveStatus {
        online,
        credentials_found,
        tabs,
    }
}

pub fn bootstrap_hive_tab(kind: &str) -> Result<String> {
    let target = target_for_kind(kind)?;
    if !target.writable {
        return Err(BooksError::from(
            "Voucher_Raw_Data is read-only. Owner cannot bootstrap the archive tab.",
        ));
    }
    let mut hive = GoogleOfficeHive::connect()?;
    hive.fail_if_missing = false;
    match hive.ensure_tab(kind, &report_headers(kind))? {
        EnsureTab::Exists => Ok(format!("{} tab is already on the sheet.", target.tab)),
        EnsureTab::BootstrappedHeaders => Ok(format!(
            "{} tab was created with headers only.",
            target.tab
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hive::MemoryHive;
    use crate::office::{save_purchase_po, PurchasePoSave};
    use crate::open_memory;

    #[test]
    fn missing_tab_keeps_local_purchase() {
        let mut books = open_memory().unwrap();
        save_purchase_po(
            &mut books,
            PurchasePoSave {
                id: None,
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "PUR-0001".into(),
                po_type: "simple".into(),
                total_value: 10.0,
                items: vec![],
                goods_received: false,
                tax_invoice_no: String::new(),
                tax_invoice_date: String::new(),
            },
        )
        .unwrap();
        let mut hive = MemoryHive::default();
        let err = submit_office_with(&books, &mut hive, KIND_PURCHASE, "PUR-0001", false).unwrap_err();
        assert!(err.to_string().contains("not on the sheet") || err.to_string().contains("missing"));
        let dirty: i64 = books
            .conn()
            .query_row(
                "SELECT is_dirty FROM purchase_po WHERE po_number = 'PUR-0001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dirty, 1);
    }

    #[test]
    fn two_pcs_same_key_second_conflicts() {
        let mut books_a = open_memory().unwrap();
        save_purchase_po(
            &mut books_a,
            PurchasePoSave {
                id: None,
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "PUR-0001".into(),
                po_type: "simple".into(),
                total_value: 10.0,
                items: vec![],
                goods_received: false,
                tax_invoice_no: String::new(),
                tax_invoice_date: String::new(),
            },
        )
        .unwrap();
        let mut hive = MemoryHive::default();
        hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE)).unwrap();
        let first = submit_office_with(&books_a, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap();
        assert!(matches!(first, CasOutcome::Ok { .. }));
        let mut books_b = open_memory().unwrap();
        save_purchase_po(
            &mut books_b,
            PurchasePoSave {
                id: None,
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "PUR-0001".into(),
                po_type: "simple".into(),
                total_value: 99.0,
                items: vec![],
                goods_received: false,
                tax_invoice_no: String::new(),
                tax_invoice_date: String::new(),
            },
        )
        .unwrap();
        match submit_office_with(&books_b, &mut hive, KIND_PURCHASE, "PUR-0001", true).unwrap() {
            CasOutcome::Conflict { key, message } => {
                assert_eq!(key, "PUR-0001");
                assert!(message.contains("just updated"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn sales_po_submits_on_po_at_project_key() {
        let mut books = open_memory().unwrap();
        crate::office::save_sales_po(
            &mut books,
            crate::office::SalesPoSave {
                id: None,
                project: "CUST_01".into(),
                po_number: "PO-1".into(),
                client: "CUST_01".into(),
                gst: String::new(),
                items: vec![],
            },
        )
        .unwrap();
        let mut hive = MemoryHive::default();
        hive.ensure_tab(KIND_SALES_PO, &tab_headers(KIND_SALES_PO)).unwrap();
        let out = submit_office_with(&books, &mut hive, KIND_SALES_PO, "PO-1@CUST_01", true).unwrap();
        assert!(matches!(out, CasOutcome::Ok { .. }));
        let dirty: i64 = books
            .conn()
            .query_row(
                "SELECT is_dirty FROM sales_po WHERE po_number = 'PO-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dirty, 0);
    }
}
