//! Submit office rows to hive tabs when those tabs exist.
//! Missing tab → keep local; owner may bootstrap headers only. Never wipe.

use crate::core::business_rules::{
    alloc_method_label, calc_po, paise_to_rupees, payment_class_label, payment_ordinal,
    purchase_pay_status, purchase_type_label, rupees_to_paise, sha256_hex, PoItemInput, TermUnit,
};
use crate::db::LocalBooks;
use crate::hive::{
    hive_conflict_message, is_live_dummy_key, sales_po_hive_key, submit_hive_row, tab_name,
    CasOutcome, EnsureTab, Hive, HiveRow, KIND_DOCUMENT, KIND_INVENTORY, KIND_LOGISTICS,
    KIND_PAYMENT, KIND_PURCHASE, KIND_SALARY, KIND_SALES_PO,
};
use crate::online::is_online;
use crate::hive_plan::{report_headers, status_targets, target_for_kind, KIND_VOUCHER_PAYMENT};
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

fn rupees_cell(n: f64) -> String {
    if n.is_finite() {
        format!("{n:.2}")
    } else {
        "0.00".into()
    }
}

fn sheet_date(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 10 {
        s[..10].to_string()
    } else {
        s.to_string()
    }
}

fn col_letter(n: usize) -> String {
    if n == 0 {
        return "A".into();
    }
    let mut n = n;
    let mut out = String::new();
    while n > 0 {
        n -= 1;
        out.insert(0, (b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    out
}

fn a1_row(row: u32, cols: usize) -> String {
    format!("A{row}:{}{row}", col_letter(cols.max(1)))
}

fn load_purchase(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let conn = books.conn();
    let (id, po_number, project, vendor, po_type, goods, tax_no, tax_date, total, created, fp, rev): (
        i64,
        String,
        String,
        String,
        String,
        i64,
        String,
        String,
        f64,
        String,
        String,
        i64,
    ) = conn
        .query_row(
            "SELECT id, po_number, COALESCE(project,''), COALESCE(vendor,''), COALESCE(type,''),
                    COALESCE(goods_received,0), COALESCE(tax_invoice_no,''), COALESCE(tax_invoice_date,''),
                    COALESCE(total_value,0), COALESCE(created_at,''), COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM purchase_po WHERE po_number = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![key],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))?;

    let mut items: Vec<PoItemInput> = Vec::new();
    let mut item_json = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT COALESCE(item_name,''), COALESCE(description,''), COALESCE(qty,0), COALESCE(rate,0),
                COALESCE(gst_pct,0), COALESCE(amount,0)
         FROM purchase_po_items WHERE po_id = ?1 ORDER BY id",
    ) {
        if let Ok(mapped) = stmt.query_map(rusqlite::params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, f64>(5)?,
            ))
        }) {
            for row in mapped.flatten() {
                item_json.push(serde_json::json!({
                    "item": row.0,
                    "description": row.1,
                    "qty": row.2,
                    "rate": row.3,
                    "gst": row.4,
                    "line": row.5,
                }));
                items.push(PoItemInput {
                    item_name: row.0,
                    description: row.1,
                    qty: row.2,
                    unit_rate_rupees: row.3,
                    gst_pct: row.4,
                });
            }
        }
    }

    let paid: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount_rupees),0) FROM purchase_payments WHERE po_number = ?1 COLLATE NOCASE",
            rusqlite::params![&po_number],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let (subtotal, gst, grand, terms) = if items.is_empty() {
        (total, 0.0, total, String::new())
    } else {
        let summary = calc_po(0.0, TermUnit::Days, paid, &items);
        (
            paise_to_rupees(summary.subtotal_paise),
            paise_to_rupees(summary.gst_paise),
            paise_to_rupees(summary.grand_total_paise),
            if summary.payment_term_days > 0 {
                format!("{} days", summary.payment_term_days)
            } else {
                String::new()
            },
        )
    };
    let status = purchase_pay_status(rupees_to_paise(grand), rupees_to_paise(paid))
        .as_str()
        .to_string();
    let notes = [
        if tax_no.is_empty() && tax_date.is_empty() {
            String::new()
        } else {
            format!("Tax {tax_no} {tax_date}")
        },
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .collect::<Vec<_>>()
    .join(" · ");

    let cells = vec![
        po_number,
        sheet_date(&created),
        vendor,
        project,
        status,
        rupees_cell(subtotal),
        rupees_cell(gst),
        rupees_cell(grand),
        rupees_cell(paid),
        rupees_cell(grand - paid),
        terms,
        notes,
        purchase_type_label(Some(&po_type)).to_string(),
        if goods != 0 { "Yes" } else { "No" }.into(),
        tax_no,
        tax_date,
        serde_json::to_string(&item_json).unwrap_or_else(|_| "[]".into()),
    ];
    Ok((cells, fp, rev))
}

fn load_payment(books: &LocalBooks, key: &str) -> Result<(Vec<String>, String, i64)> {
    let conn = books.conn();
    let (id, pay_number, po_number, vendor, amount, alloc, class, pay_date, project, remarks, fp, rev): (
        i64,
        String,
        String,
        String,
        f64,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
    ) = conn
        .query_row(
            "SELECT id, pay_number, COALESCE(po_number,''), COALESCE(vendor,''), COALESCE(amount_rupees,0),
                    COALESCE(alloc_method,''), COALESCE(pay_class,''), COALESCE(pay_date,''),
                    COALESCE(project,''), COALESCE(remarks,''), COALESCE(source_hash,''), COALESCE(hive_rev,0)
             FROM purchase_payments WHERE pay_number = ?1 COLLATE NOCASE LIMIT 1",
            rusqlite::params![key],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .map_err(|_| BooksError::from(format!("{key} is not on this PC.")))?;

    let seq: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM purchase_payments
             WHERE po_number = ?1 COLLATE NOCASE AND id <= ?2",
            rusqlite::params![&po_number, id],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let cells = vec![
        pay_number,
        payment_ordinal(seq),
        po_number,
        vendor,
        rupees_cell(amount),
        alloc_method_label(Some(&alloc)).to_string(),
        payment_class_label(Some(&class)).to_string(),
        pay_date,
        project,
        remarks,
    ];
    Ok((cells, fp, rev))
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

fn looks_like_fp(raw: &str) -> bool {
    let s = raw.trim();
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_tab_rows(values: &[Vec<String>], kind: &str) -> Vec<HiveRow> {
    let header = values.first();
    let fp_idx = header.and_then(|h| {
        h.iter()
            .position(|c| c.trim().eq_ignore_ascii_case("fp"))
    });
    let rev_idx = header.and_then(|h| {
        h.iter()
            .position(|c| c.trim().eq_ignore_ascii_case("rev"))
    });
    let mut out = Vec::new();
    for (i, row) in values.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let key = if kind == KIND_VOUCHER_PAYMENT {
            let voucher = row.first().map(|s| s.trim()).unwrap_or("");
            let n = row.get(1).map(|s| s.trim()).unwrap_or("");
            if voucher.is_empty() || n.is_empty() {
                continue;
            }
            format!("{voucher}#{n}")
        } else {
            let k = row.first().map(|s| s.trim().to_string()).unwrap_or_default();
            if k.is_empty() {
                continue;
            }
            k
        };
        let (fp, rev) = if let (Some(fi), Some(ri)) = (fp_idx, rev_idx) {
            (
                row.get(fi).cloned().unwrap_or_default(),
                row.get(ri).and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0),
            )
        } else {
            let last = row.last().cloned().unwrap_or_default();
            let second = row
                .get(row.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default();
            if looks_like_fp(&second) {
                (second, last.trim().parse::<i64>().ok().unwrap_or(0))
            } else {
                (String::new(), 0)
            }
        };
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
        let last = a1_row(*sheet_row, written.len());
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
            Ok(values) => {
                let heads = if headers.is_empty() {
                    report_headers(kind)
                } else {
                    headers.to_vec()
                };
                if !heads.is_empty() {
                    let first = values.first().cloned().unwrap_or_default();
                    let short = heads.iter().enumerate().any(|(i, h)| {
                        first.get(i).map(|s| s.trim()) != Some(h.as_str())
                    });
                    if short {
                        let range = a1_row(1, heads.len());
                        update_row_on(
                            &self.account,
                            &target.spreadsheet_id,
                            &target.tab,
                            &range,
                            &heads,
                        )?;
                    }
                }
                Ok(EnsureTab::Exists)
            }
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

impl GoogleOfficeHive {
    /// Blank a dummy test row after a live write. Refuses anything that is not a listed dummy key.
    pub fn blank_dummy_key(&mut self, kind: &str, key: &str) -> Result<()> {
        if !is_live_dummy_key(key) {
            return Err(BooksError::from(
                "Live tests may only blank dummy keys (PUR-0001, PAY-0001, CUST_01, …).",
            ));
        }
        let target = target_for_kind(kind)?;
        crate::hive_plan::refuse_raw_write(&target.spreadsheet_id, &target.tab)?;
        if !target.writable {
            return Err(BooksError::from(
                "Voucher_Raw_Data is read-only. Write the structured hive instead. The original register was not changed.",
            ));
        }
        let values = match fetch_values_on(&self.account, &target.spreadsheet_id, &target.tab) {
            Ok(v) => v,
            Err(err) if is_missing_tab_error(&err.to_string()) => return Ok(()),
            Err(err) => return Err(err),
        };
        let existing: Vec<u32> = parse_tab_rows(&values, kind)
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.key.eq_ignore_ascii_case(key))
            .map(|(i, _)| (i as u32) + 2)
            .collect();
        for sheet_row in existing {
            let a1 = a1_row(sheet_row, 26);
            let blanks = vec![String::new(); 26];
            update_row_on(
                &self.account,
                &target.spreadsheet_id,
                &target.tab,
                &a1,
                &blanks,
            )?;
        }
        Ok(())
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
    use crate::hive::{tab_headers, MemoryHive};
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
    fn purchase_cells_match_live_tally_headers() {
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
        let (cells, _, _) = office_cells(&books, KIND_PURCHASE, "PUR-0001").unwrap();
        let headers = report_headers(KIND_PURCHASE);
        assert_eq!(headers[0], "PO number");
        assert_eq!(headers[2], "Vendor");
        assert_eq!(headers[5], "Subtotal ₹");
        assert_eq!(cells[0], "PUR-0001");
        assert_eq!(cells[2], "VEND_02");
        assert_eq!(cells[3], "CUST_01");
        assert_eq!(cells[4], "Unpaid");
        assert_eq!(cells[7], "10.00");
        assert_eq!(cells[12], "Project Expenses");
        assert_eq!(cells[13], "No");
        assert_eq!(cells.len() + 2, headers.len());
    }

    #[test]
    fn payment_cells_include_first_payment_ordinal() {
        let mut books = open_memory().unwrap();
        save_purchase_po(
            &mut books,
            PurchasePoSave {
                id: None,
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "PURC:0004".into(),
                po_type: "simple".into(),
                total_value: 100.0,
                items: vec![],
                goods_received: true,
                tax_invoice_no: "INV-1".into(),
                tax_invoice_date: "2026-04-01".into(),
            },
        )
        .unwrap();
        crate::office::save_purchase_payment(
            &mut books,
            crate::office::PurchasePaymentSave {
                id: None,
                pay_number: "PAY-0010".into(),
                po_number: "PURC:0004".into(),
                vendor: "VEND_02".into(),
                amount_rupees: 40.0,
                alloc_method: "against".into(),
                pay_date: "2026-04-02".into(),
                remarks: "first against bill".into(),
            },
        )
        .unwrap();
        let (cells, _, _) = office_cells(&books, KIND_PAYMENT, "PAY-0010").unwrap();
        assert_eq!(cells[0], "PAY-0010");
        assert_eq!(cells[1], "1st payment");
        assert_eq!(cells[2], "PURC:0004");
        assert_eq!(cells[4], "40.00");
        assert_eq!(tab_name(KIND_PURCHASE), "Purchase orders");
        let target = target_for_kind(KIND_PURCHASE).unwrap();
        assert_eq!(target.tab, "Purchase orders");
        assert_eq!(target.spreadsheet_id, "15l8y0e4NRVdI0vFt4DAxNUeTvWnwnZvzvnhoKBGpgEY");
        let pay_target = target_for_kind(KIND_PAYMENT).unwrap();
        assert_eq!(pay_target.tab, "Purchase payments");
        assert_eq!(pay_target.spreadsheet_id, target.spreadsheet_id);
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
