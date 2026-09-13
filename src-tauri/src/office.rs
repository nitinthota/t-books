//! Local-only office modules. Refresh must never read or write these tables.
#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::business_rules::{
    alloc_method_for_purchase, calc_payroll, calc_po, calc_po_item, calc_salary_slip,
    classify_purchase_payment, keep_posted_pay_number, next_payment_number, next_purchase_number,
    next_salary_number, normalize_alloc_method, normalize_pay_kind, paise_to_rupees,
    purchase_pay_status, rupees_to_paise, salary_period_taken, PoItemInput, SalaryPeriodRow, TermUnit,
};
use crate::db::LocalBooks;
use crate::{BooksError, Result};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoItemIn {
    pub id: Option<i64>,
    #[serde(default)]
    pub item_name: String,
    pub description: String,
    pub qty: f64,
    pub rate: f64,
    pub gst_pct: f64,
    #[serde(default)]
    pub amount: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoPreview {
    pub items: Vec<PoItemIn>,
    pub subtotal: f64,
    pub gst: f64,
    pub grand_total: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesPo {
    pub id: i64,
    pub project: String,
    pub po_number: String,
    pub client: String,
    pub gst: String,
    pub total_value: f64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub is_dirty: bool,
    #[serde(default)]
    pub items: Vec<PoItemIn>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesPoSave {
    pub id: Option<i64>,
    pub project: String,
    pub po_number: String,
    pub client: String,
    pub gst: String,
    pub items: Vec<PoItemIn>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasePo {
    pub id: i64,
    pub project: String,
    pub vendor: String,
    pub po_number: String,
    #[serde(rename = "type")]
    pub po_type: String,
    pub total_value: f64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub goods_received: bool,
    #[serde(default)]
    pub tax_invoice_no: String,
    #[serde(default)]
    pub tax_invoice_date: String,
    #[serde(default)]
    pub is_dirty: bool,
    #[serde(default)]
    pub paid_rupees: f64,
    #[serde(default)]
    pub pay_status: String,
    #[serde(default)]
    pub items: Vec<PoItemIn>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasePoSave {
    pub id: Option<i64>,
    pub project: String,
    pub vendor: String,
    pub po_number: String,
    #[serde(rename = "type")]
    pub po_type: String,
    pub total_value: f64,
    pub items: Vec<PoItemIn>,
    #[serde(default)]
    pub goods_received: bool,
    #[serde(default)]
    pub tax_invoice_no: String,
    #[serde(default)]
    pub tax_invoice_date: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasePayment {
    pub id: Option<i64>,
    pub pay_number: String,
    pub po_number: String,
    pub vendor: String,
    pub project: String,
    pub amount_rupees: f64,
    pub alloc_method: String,
    pub pay_class: String,
    pub missing_tax_invoice: bool,
    pub pay_date: String,
    pub remarks: String,
    #[serde(default)]
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasePaymentSave {
    pub id: Option<i64>,
    pub pay_number: String,
    pub po_number: String,
    pub vendor: String,
    pub amount_rupees: f64,
    pub alloc_method: String,
    pub pay_date: String,
    pub remarks: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HrPerson {
    pub id: Option<i64>,
    pub name: String,
    pub role: String,
    pub salary: f64,
    #[serde(default = "default_active_yes")]
    pub active: String,
}

fn default_active_yes() -> String {
    "Yes".into()
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayrollRow {
    pub id: Option<i64>,
    pub person_id: i64,
    #[serde(default)]
    pub person_name: String,
    pub month: String,
    pub pf_employee: f64,
    pub pf_company: f64,
    #[serde(default)]
    pub pf_total: f64,
    pub tds: f64,
    pub total_paid: f64,
    #[serde(default)]
    pub salary_number: String,
    #[serde(default)]
    pub pay_kind: String,
    #[serde(default)]
    pub salary_rupees: f64,
    #[serde(default)]
    pub recovery_rupees: f64,
    #[serde(default)]
    pub net_rupees: f64,
    #[serde(default)]
    pub is_dirty: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryRow {
    pub id: Option<i64>,
    pub item_name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub size: String,
    pub quantity: f64,
    pub cost: f64,
    pub project: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogisticsRow {
    pub id: Option<i64>,
    pub project: String,
    pub vehicle_number: String,
    pub invoice_number: String,
    pub start_date: String,
    pub reach_date: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRow {
    pub id: Option<i64>,
    pub name: String,
    pub path: String,
    pub linked_type: String,
    pub linked_id: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorRef {
    pub vendor: String,
    pub gst: String,
}

fn trim(value: &str) -> String {
    value.trim().to_string()
}

fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn require_text(value: &str, field: &str) -> Result<String> {
    let t = trim(value);
    if t.is_empty() {
        return Err(BooksError::from(format!("{field} is required.")));
    }
    Ok(t)
}

fn valid_month(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 7
        && b[4] == b'-'
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[5..].iter().all(|c| c.is_ascii_digit())
}

fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(e, _)
            if e.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn touch_project(conn: &Connection, project: &str) -> Result<()> {
    if project.is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO projects (project, updated_at) VALUES (?1, datetime('now'))
         ON CONFLICT(project) DO UPDATE SET updated_at = excluded.updated_at",
        params![project],
    )?;
    Ok(())
}

fn touch_vendor(conn: &Connection, vendor: &str) -> Result<()> {
    if vendor.is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO vendors (vendor, updated_at) VALUES (?1, datetime('now'))
         ON CONFLICT(vendor) DO NOTHING",
        params![vendor],
    )?;
    Ok(())
}

fn to_po_input(item: &PoItemIn) -> PoItemInput {
    PoItemInput {
        item_name: item.item_name.clone(),
        description: item.description.clone(),
        qty: finite(item.qty),
        unit_rate_rupees: finite(item.rate),
        gst_pct: finite(item.gst_pct),
    }
}

pub fn preview_po(items: &[PoItemIn]) -> PoPreview {
    let input: Vec<PoItemInput> = items.iter().map(to_po_input).collect();
    let summary = calc_po(0.0, TermUnit::Days, 0.0, &input);
    let out_items: Vec<PoItemIn> = items
        .iter()
        .zip(summary.items.iter())
        .map(|(src, calc)| PoItemIn {
            id: src.id,
            item_name: if src.item_name.trim().is_empty() {
                calc.item_name.clone()
            } else {
                src.item_name.clone()
            },
            description: calc.description.clone(),
            qty: calc.qty,
            rate: src.rate,
            gst_pct: calc.gst_pct,
            amount: paise_to_rupees(calc.total_cost_paise),
        })
        .collect();
    PoPreview {
        items: out_items,
        subtotal: paise_to_rupees(summary.subtotal_paise),
        gst: paise_to_rupees(summary.gst_paise),
        grand_total: paise_to_rupees(summary.grand_total_paise),
    }
}

fn load_items(conn: &Connection, table: &str, po_id: i64) -> Result<Vec<PoItemIn>> {
    let sql = format!(
        "SELECT id, COALESCE(item_name,''), description, qty, rate, gst_pct, amount FROM {table} WHERE po_id = ?1 ORDER BY id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![po_id], |row| {
        Ok(PoItemIn {
            id: Some(row.get(0)?),
            item_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            qty: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            rate: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
            gst_pct: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
            amount: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

fn replace_items(conn: &Connection, table: &str, po_id: i64, items: &[PoItemIn]) -> Result<()> {
    conn.execute(&format!("DELETE FROM {table} WHERE po_id = ?1"), params![po_id])?;
    let sql = format!(
        "INSERT INTO {table} (po_id, item_name, description, qty, rate, gst_pct, amount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    );
    let mut stmt = conn.prepare(&sql)?;
    for item in items {
        let calc = calc_po_item(&to_po_input(item));
        let name = if item.item_name.trim().is_empty() {
            calc.item_name.clone()
        } else {
            trim(&item.item_name)
        };
        stmt.execute(params![
            po_id,
            name,
            calc.description,
            calc.qty,
            finite(item.rate),
            calc.gst_pct,
            paise_to_rupees(calc.total_cost_paise),
        ])?;
    }
    Ok(())
}

fn po_exists(
    conn: &Connection,
    table: &str,
    project: &str,
    po_number: &str,
    skip_id: Option<i64>,
) -> Result<bool> {
    let sql = format!(
        "SELECT id FROM {table} WHERE project = ?1 COLLATE NOCASE AND po_number = ?2 COLLATE NOCASE LIMIT 1"
    );
    let id: Option<i64> = conn
        .query_row(&sql, params![project, po_number], |row| row.get(0))
        .optional()?;
    Ok(match id {
        Some(found) => skip_id != Some(found),
        None => false,
    })
}

fn map_sales(row: &rusqlite::Row<'_>) -> rusqlite::Result<SalesPo> {
    Ok(SalesPo {
        id: row.get(0)?,
        project: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        po_number: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        client: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        gst: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        total_value: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
        created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        updated_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        is_dirty: row.get::<_, i64>(8).unwrap_or(0) != 0,
        items: Vec::new(),
    })
}

pub fn list_sales_po(books: &LocalBooks) -> Result<Vec<SalesPo>> {
    let mut stmt = books.conn().prepare(
        "SELECT id, project, po_number, client, gst, total_value, created_at, updated_at,
                COALESCE(is_dirty,0)
         FROM sales_po ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], map_sales)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_sales_po(books: &LocalBooks, id: i64) -> Result<SalesPo> {
    let mut po = books
        .conn()
        .query_row(
            "SELECT id, project, po_number, client, gst, total_value, created_at, updated_at,
                    COALESCE(is_dirty,0)
             FROM sales_po WHERE id = ?1",
            params![id],
            map_sales,
        )
        .optional()?
        .ok_or_else(|| BooksError::from("That sales PO was not found on this PC."))?;
    po.items = load_items(books.conn(), "sales_po_items", id)?;
    Ok(po)
}

pub fn save_sales_po(books: &mut LocalBooks, payload: SalesPoSave) -> Result<SalesPo> {
    let project = require_text(&payload.project, "Project")?;
    let po_number = require_text(&payload.po_number, "PO number")?;
    let client = trim(&payload.client);
    let gst = trim(&payload.gst);
    if po_exists(
        books.conn(),
        "sales_po",
        &project,
        &po_number,
        payload.id,
    )? {
        return Err(BooksError::from(
            "A sales PO with this number already exists on this project.",
        ));
    }
    let preview = preview_po(&payload.items);
    let total = preview.grand_total;
    let tx = books.conn_mut().transaction()?;
    touch_project(&tx, &project)?;
    let id = if let Some(existing) = payload.id {
        let changed = tx.execute(
            "UPDATE sales_po SET project = ?1, po_number = ?2, client = ?3, gst = ?4,
             total_value = ?5, is_dirty = 1, updated_at = datetime('now') WHERE id = ?6",
            params![project, po_number, client, gst, total, existing],
        )?;
        if changed == 0 {
            return Err(BooksError::from("That sales PO was not found on this PC."));
        }
        existing
    } else {
        tx.execute(
            "INSERT INTO sales_po (project, po_number, client, gst, total_value, is_dirty, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'), datetime('now'))",
            params![project, po_number, client, gst, total],
        )?;
        tx.last_insert_rowid()
    };
    replace_items(&tx, "sales_po_items", id, &payload.items)?;
    tx.commit().map_err(|err| {
        if is_unique_violation(&err) {
            BooksError::from("A sales PO with this number already exists on this project.")
        } else {
            err.into()
        }
    })?;
    get_sales_po(books, id)
}

pub fn delete_sales_po(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM sales_po WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That sales PO was not found on this PC."));
    }
    Ok(())
}

fn paid_for_po(conn: &Connection, po_number: &str) -> f64 {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_rupees), 0) FROM purchase_payments WHERE po_number = ?1 COLLATE NOCASE",
        params![po_number],
        |row| row.get::<_, f64>(0),
    )
    .unwrap_or(0.0)
}

fn list_purchase_numbers(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT po_number FROM purchase_po WHERE TRIM(po_number) != ''")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn list_pay_numbers(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT pay_number FROM purchase_payments WHERE TRIM(pay_number) != ''")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn list_salary_numbers(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT salary_number FROM hr_payroll WHERE TRIM(COALESCE(salary_number,'')) != ''",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn po_number_taken(conn: &Connection, po_number: &str, skip_id: Option<i64>) -> Result<bool> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM purchase_po WHERE po_number = ?1 COLLATE NOCASE LIMIT 1",
            params![po_number],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match id {
        Some(found) => skip_id != Some(found),
        None => false,
    })
}

fn map_purchase(row: &rusqlite::Row<'_>) -> rusqlite::Result<PurchasePo> {
    let po_number: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
    let total_value: f64 = row.get::<_, Option<f64>>(5)?.unwrap_or(0.0);
    let tax_invoice_no: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
    let tax_invoice_date: String = row.get::<_, Option<String>>(10)?.unwrap_or_default();
    Ok(PurchasePo {
        id: row.get(0)?,
        project: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        vendor: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        po_number,
        po_type: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| "contract".into()),
        total_value,
        created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        updated_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        goods_received: row.get::<_, i64>(8).unwrap_or(0) != 0,
        tax_invoice_no,
        tax_invoice_date,
        is_dirty: row.get::<_, i64>(11).unwrap_or(0) != 0,
        paid_rupees: 0.0,
        pay_status: String::new(),
        items: Vec::new(),
    })
}

fn decorate_purchase(conn: &Connection, mut po: PurchasePo) -> PurchasePo {
    po.paid_rupees = paid_for_po(conn, &po.po_number);
    po.pay_status = purchase_pay_status(
        rupees_to_paise(po.total_value),
        rupees_to_paise(po.paid_rupees),
    )
    .as_str()
    .into();
    po
}

pub fn list_purchase_po(books: &LocalBooks) -> Result<Vec<PurchasePo>> {
    let mut stmt = books.conn().prepare(
        "SELECT id, project, vendor, po_number, type, total_value, created_at, updated_at,
                COALESCE(goods_received,0), COALESCE(tax_invoice_no,''), COALESCE(tax_invoice_date,''),
                COALESCE(is_dirty,0)
         FROM purchase_po ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], map_purchase)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(decorate_purchase(books.conn(), row?));
    }
    Ok(out)
}

pub fn get_purchase_po(books: &LocalBooks, id: i64) -> Result<PurchasePo> {
    let mut po = books
        .conn()
        .query_row(
            "SELECT id, project, vendor, po_number, type, total_value, created_at, updated_at,
                    COALESCE(goods_received,0), COALESCE(tax_invoice_no,''), COALESCE(tax_invoice_date,''),
                    COALESCE(is_dirty,0)
             FROM purchase_po WHERE id = ?1",
            params![id],
            map_purchase,
        )
        .optional()?
        .ok_or_else(|| BooksError::from("That purchase PO was not found on this PC."))?;
    po.items = load_items(books.conn(), "purchase_po_items", id)?;
    Ok(decorate_purchase(books.conn(), po))
}

pub fn save_purchase_po(books: &mut LocalBooks, payload: PurchasePoSave) -> Result<PurchasePo> {
    let project = require_text(&payload.project, "Project")?;
    let vendor = require_text(&payload.vendor, "Vendor")?;
    let po_type = if trim(&payload.po_type).eq_ignore_ascii_case("simple") {
        "simple"
    } else {
        "contract"
    };
    let existing_number = if let Some(id) = payload.id {
        books
            .conn()
            .query_row(
                "SELECT po_number FROM purchase_po WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_default()
    } else {
        String::new()
    };
    let requested = trim(&payload.po_number);
    let po_number = if !existing_number.is_empty() {
        keep_posted_pay_number(&existing_number, &requested)
    } else if requested.is_empty() {
        next_purchase_number(&list_purchase_numbers(books.conn())?)
    } else {
        requested
    };
    if po_number_taken(books.conn(), &po_number, payload.id)? {
        return Err(BooksError::from(
            "A purchase bill with this number already exists.",
        ));
    }
    if po_exists(
        books.conn(),
        "purchase_po",
        &project,
        &po_number,
        payload.id,
    )? {
        return Err(BooksError::from(
            "A purchase PO with this number already exists on this project.",
        ));
    }
    let (total, items): (f64, Vec<PoItemIn>) = if po_type == "simple" {
        (finite(payload.total_value), Vec::new())
    } else {
        let preview = preview_po(&payload.items);
        (preview.grand_total, payload.items.clone())
    };
    let goods = if payload.goods_received { 1i64 } else { 0 };
    let tax_no = trim(&payload.tax_invoice_no);
    let tax_date = trim(&payload.tax_invoice_date);
    let tx = books.conn_mut().transaction()?;
    touch_project(&tx, &project)?;
    touch_vendor(&tx, &vendor)?;
    let id = if let Some(existing) = payload.id {
        let changed = tx.execute(
            "UPDATE purchase_po SET project = ?1, vendor = ?2, po_number = ?3, type = ?4,
             total_value = ?5, goods_received = ?6, tax_invoice_no = ?7, tax_invoice_date = ?8,
             is_dirty = 1, updated_at = datetime('now') WHERE id = ?9",
            params![
                project, vendor, po_number, po_type, total, goods, tax_no, tax_date, existing
            ],
        )?;
        if changed == 0 {
            return Err(BooksError::from(
                "That purchase PO was not found on this PC.",
            ));
        }
        existing
    } else {
        tx.execute(
            "INSERT INTO purchase_po (project, vendor, po_number, type, total_value, goods_received,
             tax_invoice_no, tax_invoice_date, is_dirty, hive_rev, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 0, datetime('now'), datetime('now'))",
            params![project, vendor, po_number, po_type, total, goods, tax_no, tax_date],
        )?;
        tx.last_insert_rowid()
    };
    replace_items(&tx, "purchase_po_items", id, &items)?;
    tx.commit().map_err(|err| {
        if is_unique_violation(&err) {
            BooksError::from("A purchase bill with this number already exists.")
        } else {
            err.into()
        }
    })?;
    get_purchase_po(books, id)
}

pub fn delete_purchase_po(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM purchase_po WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from(
            "That purchase PO was not found on this PC.",
        ));
    }
    Ok(())
}

fn map_payment(row: &rusqlite::Row<'_>) -> rusqlite::Result<PurchasePayment> {
    Ok(PurchasePayment {
        id: Some(row.get(0)?),
        pay_number: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        po_number: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        vendor: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        project: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        amount_rupees: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
        alloc_method: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        pay_class: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        missing_tax_invoice: row.get::<_, i64>(8).unwrap_or(0) != 0,
        pay_date: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        remarks: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
        is_dirty: row.get::<_, i64>(11).unwrap_or(0) != 0,
    })
}

const PAYMENT_SELECT: &str = "SELECT id, pay_number, po_number, vendor, project, amount_rupees,
        alloc_method, pay_class, COALESCE(missing_tax_invoice,0), COALESCE(pay_date,''),
        COALESCE(remarks,''), COALESCE(is_dirty,0)
 FROM purchase_payments";

pub fn list_purchase_payments(
    books: &LocalBooks,
    po_number: Option<&str>,
) -> Result<Vec<PurchasePayment>> {
    let po = po_number.unwrap_or("").trim();
    if po.is_empty() {
        let sql = format!("{PAYMENT_SELECT} ORDER BY pay_number COLLATE NOCASE, id");
        let mut stmt = books.conn().prepare(&sql)?;
        let rows = stmt.query_map([], map_payment)?;
        return rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into);
    }
    let sql = format!(
        "{PAYMENT_SELECT} WHERE po_number = ?1 COLLATE NOCASE ORDER BY pay_number COLLATE NOCASE, id"
    );
    let mut stmt = books.conn().prepare(&sql)?;
    let rows = stmt.query_map(params![po], map_payment)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn save_purchase_payment(
    books: &mut LocalBooks,
    payload: PurchasePaymentSave,
) -> Result<PurchasePayment> {
    let po_number = require_text(&payload.po_number, "Purchase number")?;
    let amount = finite(payload.amount_rupees);
    if amount <= 0.0 {
        return Err(BooksError::from("Payment amount must be greater than zero."));
    }
    let po = books
        .conn()
        .query_row(
            "SELECT vendor, project, total_value, COALESCE(goods_received,0),
                    COALESCE(tax_invoice_no,''), COALESCE(tax_invoice_date,'')
             FROM purchase_po WHERE po_number = ?1 COLLATE NOCASE LIMIT 1",
            params![po_number],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                    row.get::<_, i64>(3).unwrap_or(0) != 0,
                    row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                ))
            },
        )
        .optional()?
        .ok_or_else(|| BooksError::from("That purchase bill was not found on this PC."))?;
    let (po_vendor, project, grand, goods, tax_no, tax_date) = po;
    let vendor = if trim(&payload.vendor).is_empty() {
        po_vendor
    } else {
        trim(&payload.vendor)
    };
    let existing_number = if let Some(id) = payload.id {
        books
            .conn()
            .query_row(
                "SELECT pay_number FROM purchase_payments WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_default()
    } else {
        String::new()
    };
    let requested = trim(&payload.pay_number);
    let pay_number = if !existing_number.is_empty() {
        keep_posted_pay_number(&existing_number, &requested)
    } else if requested.is_empty() {
        next_payment_number(&list_pay_numbers(books.conn())?, Some(&po_number))
    } else {
        requested
    };
    let skip_id = payload.id.unwrap_or(-1);
    let paid_before: f64 = books.conn().query_row(
        "SELECT COALESCE(SUM(amount_rupees),0) FROM purchase_payments
         WHERE po_number = ?1 COLLATE NOCASE AND id != ?2",
        params![po_number, skip_id],
        |row| row.get(0),
    )?;
    let missing_tax = crate::core::business_rules::tax_invoice_missing(
        Some(tax_no.as_str()),
        Some(tax_date.as_str()),
    );
    let (class, missing) = classify_purchase_payment(
        goods,
        missing_tax,
        rupees_to_paise(grand),
        rupees_to_paise(paid_before),
        rupees_to_paise(amount),
    );
    let alloc = if trim(&payload.alloc_method).is_empty() {
        alloc_method_for_purchase(goods)
    } else {
        normalize_alloc_method(Some(payload.alloc_method.as_str()))
    };
    let pay_date = trim(&payload.pay_date);
    let remarks = trim(&payload.remarks);
    let missing_i = if missing { 1i64 } else { 0 };
    let id = if let Some(existing) = payload.id {
        let n = books.conn().execute(
            "UPDATE purchase_payments SET pay_number = ?1, po_number = ?2, vendor = ?3, project = ?4,
             amount_rupees = ?5, alloc_method = ?6, pay_class = ?7, missing_tax_invoice = ?8,
             pay_date = ?9, remarks = ?10, is_dirty = 1, updated_at = datetime('now') WHERE id = ?11",
            params![
                pay_number,
                po_number,
                vendor,
                project,
                amount,
                alloc.as_str(),
                class.as_str(),
                missing_i,
                pay_date,
                remarks,
                existing
            ],
        )?;
        if n == 0 {
            return Err(BooksError::from("That payment was not found on this PC."));
        }
        existing
    } else {
        books.conn().execute(
            "INSERT INTO purchase_payments (pay_number, po_number, vendor, project, amount_rupees,
             alloc_method, pay_class, missing_tax_invoice, pay_date, remarks, is_dirty, hive_rev,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, 0, datetime('now'), datetime('now'))",
            params![
                pay_number,
                po_number,
                vendor,
                project,
                amount,
                alloc.as_str(),
                class.as_str(),
                missing_i,
                pay_date,
                remarks
            ],
        )?;
        books.conn().last_insert_rowid()
    };
    let sql = format!("{PAYMENT_SELECT} WHERE id = ?1");
    books
        .conn()
        .query_row(&sql, params![id], map_payment)
        .map_err(Into::into)
}

pub fn delete_purchase_payment(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM purchase_payments WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That payment was not found on this PC."));
    }
    Ok(())
}

fn map_person(row: &rusqlite::Row<'_>) -> rusqlite::Result<HrPerson> {
    Ok(HrPerson {
        id: Some(row.get(0)?),
        name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        role: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        salary: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
        active: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| "Yes".into()),
    })
}

pub fn list_hr_people(books: &LocalBooks) -> Result<Vec<HrPerson>> {
    let mut stmt = books.conn().prepare(
        "SELECT id, name, role, salary, COALESCE(active,'Yes') FROM hr_people ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = stmt.query_map([], map_person)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn save_hr_person(books: &LocalBooks, payload: HrPerson) -> Result<HrPerson> {
    let name = require_text(&payload.name, "Name")?;
    let role = trim(&payload.role);
    let salary = finite(payload.salary);
    let active = match trim(&payload.active).to_ascii_lowercase().as_str() {
        "no" | "n" | "false" | "0" => "No".to_string(),
        _ => "Yes".to_string(),
    };
    if active == "Yes" && salary <= 0.0 {
        return Err(BooksError::from("Salary is required when a person is Active."));
    }
    let id = if let Some(existing) = payload.id {
        let n = books.conn().execute(
            "UPDATE hr_people SET name = ?1, role = ?2, salary = ?3, active = ?4 WHERE id = ?5",
            params![name, role, salary, active, existing],
        )?;
        if n == 0 {
            return Err(BooksError::from("That person was not found on this PC."));
        }
        existing
    } else {
        books.conn().execute(
            "INSERT INTO hr_people (name, role, salary, active) VALUES (?1, ?2, ?3, ?4)",
            params![name, role, salary, active],
        )?;
        books.conn().last_insert_rowid()
    };
    books
        .conn()
        .query_row(
            "SELECT id, name, role, salary, COALESCE(active,'Yes') FROM hr_people WHERE id = ?1",
            params![id],
            map_person,
        )
        .map_err(Into::into)
}

pub fn delete_hr_person(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM hr_people WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That person was not found on this PC."));
    }
    Ok(())
}

fn payroll_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PayrollRow> {
    let pf_employee: f64 = row.get::<_, Option<f64>>(4)?.unwrap_or(0.0);
    let pf_company: f64 = row.get::<_, Option<f64>>(5)?.unwrap_or(0.0);
    let tds: f64 = row.get::<_, Option<f64>>(6)?.unwrap_or(0.0);
    let salary_rupees: f64 = row.get::<_, Option<f64>>(11)?.unwrap_or(0.0);
    let recovery_rupees: f64 = row.get::<_, Option<f64>>(12)?.unwrap_or(0.0);
    let pay_kind_raw: String = row.get::<_, Option<String>>(10)?.unwrap_or_default();
    let kind = normalize_pay_kind(Some(pay_kind_raw.as_str()));
    let slip = calc_salary_slip(
        kind,
        rupees_to_paise(salary_rupees),
        rupees_to_paise(pf_employee),
        rupees_to_paise(pf_company),
        rupees_to_paise(tds),
        rupees_to_paise(recovery_rupees),
        rupees_to_paise(row.get::<_, Option<f64>>(7)?.unwrap_or(0.0)),
    );
    let calc = calc_payroll(pf_company, pf_employee, tds);
    Ok(PayrollRow {
        id: Some(row.get(0)?),
        person_id: row.get(1)?,
        person_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        month: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        pf_employee,
        pf_company,
        pf_total: paise_to_rupees(calc.pf_total_paise),
        tds,
        total_paid: paise_to_rupees(slip.net_paise),
        salary_number: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        pay_kind: kind.as_str().into(),
        salary_rupees,
        recovery_rupees: paise_to_rupees(slip.recovery_paise),
        net_rupees: paise_to_rupees(slip.net_paise),
        is_dirty: row.get::<_, i64>(13).unwrap_or(0) != 0,
    })
}

const PAYROLL_SELECT: &str = "SELECT p.id, p.person_id, h.name, p.month, p.pf_employee, p.pf_company, p.tds, p.total_paid,
        h.salary, COALESCE(p.salary_number,''), COALESCE(p.pay_kind,'salary'),
        COALESCE(p.salary_rupees,0), COALESCE(p.recovery_rupees,0), COALESCE(p.is_dirty,0)
 FROM hr_payroll p JOIN hr_people h ON h.id = p.person_id";

pub fn list_hr_payroll(books: &LocalBooks) -> Result<Vec<PayrollRow>> {
    let sql = format!("{PAYROLL_SELECT} ORDER BY p.month DESC, h.name COLLATE NOCASE");
    let mut stmt = books.conn().prepare(&sql)?;
    let rows = stmt.query_map([], payroll_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn parse_month_parts(month: &str) -> Option<(i32, i32)> {
    if !valid_month(month) {
        return None;
    }
    let year: i32 = month[..4].parse().ok()?;
    let m: i32 = month[5..7].parse().ok()?;
    Some((m, year))
}

pub fn save_hr_payroll(books: &LocalBooks, payload: PayrollRow) -> Result<PayrollRow> {
    if payload.person_id <= 0 {
        return Err(BooksError::from("Person is required."));
    }
    let month = trim(&payload.month);
    if !valid_month(&month) {
        return Err(BooksError::from("Month must be YYYY-MM."));
    }
    let person: (String, f64, String) = books
        .conn()
        .query_row(
            "SELECT name, salary, COALESCE(active,'Yes') FROM hr_people WHERE id = ?1",
            params![payload.person_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                    row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "Yes".into()),
                ))
            },
        )
        .optional()?
        .ok_or_else(|| BooksError::from("That person was not found on this PC."))?;
    let (person_name, person_salary, active) = person;
    let kind = normalize_pay_kind(Some(payload.pay_kind.as_str()));
    let (period_month, period_year) = parse_month_parts(&month)
        .ok_or_else(|| BooksError::from("Month must be YYYY-MM."))?;
    if kind == crate::core::business_rules::PayKind::Salary {
        let mut existing_rows = Vec::new();
        {
            let mut stmt = books.conn().prepare(
                "SELECT id, person_id, pay_kind, month FROM hr_payroll WHERE pay_kind IS NULL OR pay_kind != 'advance'",
            )?;
            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let pid: i64 = row.get(1)?;
                let pk: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
                let mo: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
                let (m, y) = parse_month_parts(&mo).unwrap_or((0, 0));
                Ok(SalaryPeriodRow {
                    id: Some(id.to_string()),
                    employee_id: Some(pid.to_string()),
                    employee_name: String::new(),
                    pay_kind: Some(pk),
                    period_month: m,
                    period_year: y,
                })
            })?;
            for row in rows {
                existing_rows.push(row?);
            }
        }
        let input = SalaryPeriodRow {
            id: payload.id.map(|id| id.to_string()),
            employee_id: Some(payload.person_id.to_string()),
            employee_name: person_name.clone(),
            pay_kind: Some(kind.as_str().into()),
            period_month,
            period_year,
        };
        if salary_period_taken(&existing_rows, &input) {
            return Err(BooksError::from(
                "Payroll for that person already exists in this month.",
            ));
        }
    }
    let salary_rupees = if payload.salary_rupees > 0.0 {
        finite(payload.salary_rupees)
    } else {
        person_salary
    };
    if kind == crate::core::business_rules::PayKind::Salary
        && active.eq_ignore_ascii_case("Yes")
        && salary_rupees <= 0.0
    {
        return Err(BooksError::from("Salary is required when a person is Active."));
    }
    let pf_employee = finite(payload.pf_employee);
    let pf_company = finite(payload.pf_company);
    let tds = finite(payload.tds);
    let recovery = finite(payload.recovery_rupees);
    let slip = calc_salary_slip(
        kind,
        rupees_to_paise(salary_rupees),
        rupees_to_paise(pf_employee),
        rupees_to_paise(pf_company),
        rupees_to_paise(tds),
        rupees_to_paise(recovery),
        rupees_to_paise(finite(payload.total_paid)),
    );
    let total_paid = paise_to_rupees(slip.net_paise);
    let net = paise_to_rupees(slip.net_paise);
    let existing_number = if let Some(id) = payload.id {
        books
            .conn()
            .query_row(
                "SELECT COALESCE(salary_number,'') FROM hr_payroll WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_default()
    } else {
        String::new()
    };
    let requested = trim(&payload.salary_number);
    let salary_number = if !existing_number.is_empty() {
        keep_posted_pay_number(&existing_number, &requested)
    } else if requested.is_empty() {
        next_salary_number(&list_salary_numbers(books.conn())?)
    } else {
        requested
    };
    let id = if let Some(existing) = payload.id {
        let n = books.conn().execute(
            "UPDATE hr_payroll SET person_id = ?1, month = ?2, pf_employee = ?3, pf_company = ?4,
             tds = ?5, total_paid = ?6, salary_number = ?7, pay_kind = ?8, salary_rupees = ?9,
             recovery_rupees = ?10, net_rupees = ?11, is_dirty = 1 WHERE id = ?12",
            params![
                payload.person_id,
                month,
                pf_employee,
                pf_company,
                tds,
                total_paid,
                salary_number,
                kind.as_str(),
                salary_rupees,
                paise_to_rupees(slip.recovery_paise),
                net,
                existing
            ],
        )?;
        if n == 0 {
            return Err(BooksError::from("That payroll row was not found on this PC."));
        }
        existing
    } else {
        books.conn().execute(
            "INSERT INTO hr_payroll (person_id, month, pf_employee, pf_company, tds, total_paid,
             salary_number, pay_kind, salary_rupees, recovery_rupees, net_rupees, is_dirty, hive_rev)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, 0)",
            params![
                payload.person_id,
                month,
                pf_employee,
                pf_company,
                tds,
                total_paid,
                salary_number,
                kind.as_str(),
                salary_rupees,
                paise_to_rupees(slip.recovery_paise),
                net
            ],
        )?;
        books.conn().last_insert_rowid()
    };
    let sql = format!("{PAYROLL_SELECT} WHERE p.id = ?1");
    books
        .conn()
        .query_row(&sql, params![id], payroll_from_row)
        .map_err(Into::into)
}

pub fn delete_hr_payroll(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM hr_payroll WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That payroll row was not found on this PC."));
    }
    Ok(())
}

fn map_inventory(row: &rusqlite::Row<'_>) -> rusqlite::Result<InventoryRow> {
    Ok(InventoryRow {
        id: Some(row.get(0)?),
        item_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        item_type: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        size: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        quantity: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
        cost: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
        project: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
    })
}

pub fn list_inventory(books: &LocalBooks, query: Option<&str>) -> Result<Vec<InventoryRow>> {
    let q = query.unwrap_or("").trim();
    if q.is_empty() {
        let mut stmt = books.conn().prepare(
            "SELECT id, item_name, type, size, quantity, cost, project FROM inventory ORDER BY item_name COLLATE NOCASE, id",
        )?;
        let rows = stmt.query_map([], map_inventory)?;
        return rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into);
    }
    let like = format!("%{}%", q);
    let mut stmt = books.conn().prepare(
        "SELECT id, item_name, type, size, quantity, cost, project FROM inventory
         WHERE item_name LIKE ?1 ESCAPE '\\' OR type LIKE ?1 ESCAPE '\\'
         ORDER BY item_name COLLATE NOCASE, id",
    )?;
    let rows = stmt.query_map(params![like], map_inventory)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn save_inventory(books: &LocalBooks, payload: InventoryRow) -> Result<InventoryRow> {
    let item_name = require_text(&payload.item_name, "Item name")?;
    let item_type = trim(&payload.item_type);
    let size = trim(&payload.size);
    let project = trim(&payload.project);
    let quantity = finite(payload.quantity);
    let cost = finite(payload.cost);
    touch_project(books.conn(), &project)?;
    let id = if let Some(existing) = payload.id {
        let n = books.conn().execute(
            "UPDATE inventory SET item_name = ?1, type = ?2, size = ?3, quantity = ?4, cost = ?5, project = ?6, is_dirty = 1 WHERE id = ?7",
            params![item_name, item_type, size, quantity, cost, project, existing],
        )?;
        if n == 0 {
            return Err(BooksError::from("That inventory row was not found on this PC."));
        }
        existing
    } else {
        books.conn().execute(
            "INSERT INTO inventory (item_name, type, size, quantity, cost, project, is_dirty) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            params![item_name, item_type, size, quantity, cost, project],
        )?;
        books.conn().last_insert_rowid()
    };
    books
        .conn()
        .query_row(
            "SELECT id, item_name, type, size, quantity, cost, project FROM inventory WHERE id = ?1",
            params![id],
            map_inventory,
        )
        .map_err(Into::into)
}

pub fn delete_inventory(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM inventory WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That inventory row was not found on this PC."));
    }
    Ok(())
}

pub fn move_inventory(books: &LocalBooks, id: i64, project: &str) -> Result<InventoryRow> {
    let mut row = books
        .conn()
        .query_row(
            "SELECT id, item_name, type, size, quantity, cost, project FROM inventory WHERE id = ?1",
            params![id],
            map_inventory,
        )
        .optional()?
        .ok_or_else(|| BooksError::from("That inventory row was not found on this PC."))?;
    row.project = trim(project);
    save_inventory(books, row)
}

fn map_logistics(row: &rusqlite::Row<'_>) -> rusqlite::Result<LogisticsRow> {
    Ok(LogisticsRow {
        id: Some(row.get(0)?),
        project: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        vehicle_number: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        invoice_number: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        start_date: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        reach_date: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
    })
}

pub fn list_logistics(books: &LocalBooks) -> Result<Vec<LogisticsRow>> {
    let mut stmt = books.conn().prepare(
        "SELECT id, project, vehicle_number, invoice_number, start_date, reach_date
         FROM logistics ORDER BY start_date DESC, id DESC",
    )?;
    let rows = stmt.query_map([], map_logistics)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn save_logistics(books: &LocalBooks, payload: LogisticsRow) -> Result<LogisticsRow> {
    let vehicle_number = require_text(&payload.vehicle_number, "Vehicle number")?;
    let project = trim(&payload.project);
    let invoice_number = trim(&payload.invoice_number);
    let start_date = trim(&payload.start_date);
    let reach_date = trim(&payload.reach_date);
    touch_project(books.conn(), &project)?;
    let id = if let Some(existing) = payload.id {
        let n = books.conn().execute(
            "UPDATE logistics SET project = ?1, vehicle_number = ?2, invoice_number = ?3, start_date = ?4, reach_date = ?5, is_dirty = 1 WHERE id = ?6",
            params![project, vehicle_number, invoice_number, start_date, reach_date, existing],
        )?;
        if n == 0 {
            return Err(BooksError::from("That logistics row was not found on this PC."));
        }
        existing
    } else {
        books.conn().execute(
            "INSERT INTO logistics (project, vehicle_number, invoice_number, start_date, reach_date, is_dirty)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![project, vehicle_number, invoice_number, start_date, reach_date],
        )?;
        books.conn().last_insert_rowid()
    };
    books
        .conn()
        .query_row(
            "SELECT id, project, vehicle_number, invoice_number, start_date, reach_date FROM logistics WHERE id = ?1",
            params![id],
            map_logistics,
        )
        .map_err(Into::into)
}

pub fn delete_logistics(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM logistics WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That logistics row was not found on this PC."));
    }
    Ok(())
}

pub fn move_logistics(books: &LocalBooks, id: i64, project: &str) -> Result<LogisticsRow> {
    let mut row = books
        .conn()
        .query_row(
            "SELECT id, project, vehicle_number, invoice_number, start_date, reach_date FROM logistics WHERE id = ?1",
            params![id],
            map_logistics,
        )
        .optional()?
        .ok_or_else(|| BooksError::from("That logistics row was not found on this PC."))?;
    row.project = trim(project);
    save_logistics(books, row)
}

fn map_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentRow> {
    Ok(DocumentRow {
        id: Some(row.get(0)?),
        name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        path: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        linked_type: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        linked_id: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        created_at: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
    })
}

pub fn list_documents(books: &LocalBooks) -> Result<Vec<DocumentRow>> {
    let mut stmt = books.conn().prepare(
        "SELECT id, name, path, linked_type, linked_id, created_at FROM documents ORDER BY created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], map_document)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn save_document(books: &LocalBooks, payload: DocumentRow) -> Result<DocumentRow> {
    let name = trim(&payload.name);
    let path = trim(&payload.path);
    if name.is_empty() && path.is_empty() {
        return Err(BooksError::from("Name or file path is required."));
    }
    let linked_type = match trim(&payload.linked_type).to_ascii_lowercase().as_str() {
        "" => String::new(),
        "voucher" => "voucher".into(),
        "project" => "project".into(),
        "po" => "po".into(),
        other => {
            return Err(BooksError::from(format!(
                "Link type must be voucher, project, or PO (got {other})."
            )));
        }
    };
    let linked_id = trim(&payload.linked_id);
    let stored_name = if name.is_empty() { path.clone() } else { name };
    let id = if let Some(existing) = payload.id {
        let n = books.conn().execute(
            "UPDATE documents SET name = ?1, path = ?2, linked_type = ?3, linked_id = ?4, is_dirty = 1 WHERE id = ?5",
            params![stored_name, path, linked_type, linked_id, existing],
        )?;
        if n == 0 {
            return Err(BooksError::from("That document was not found on this PC."));
        }
        existing
    } else {
        books.conn().execute(
            "INSERT INTO documents (name, path, linked_type, linked_id, created_at, is_dirty)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), 1)",
            params![stored_name, path, linked_type, linked_id],
        )?;
        books.conn().last_insert_rowid()
    };
    books
        .conn()
        .query_row(
            "SELECT id, name, path, linked_type, linked_id, created_at FROM documents WHERE id = ?1",
            params![id],
            map_document,
        )
        .map_err(Into::into)
}

pub fn delete_document(books: &LocalBooks, id: i64) -> Result<()> {
    let n = books
        .conn()
        .execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(BooksError::from("That document was not found on this PC."));
    }
    Ok(())
}

fn open_path(path: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| BooksError::from(format!("Could not open that file: {e}")))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let candidate = std::path::Path::new(path);
        if !candidate.exists() {
            return Err(BooksError::from(format!(
                "No file at {path}. T Books stores the path only and does not upload it."
            )));
        }
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        Ok(())
    }
}

pub fn open_document(books: &LocalBooks, id: i64) -> Result<String> {
    let path: String = books
        .conn()
        .query_row(
            "SELECT path FROM documents WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .unwrap_or_default();
    let path = trim(&path);
    if path.is_empty() {
        return Err(BooksError::from("That document has no file path on this PC."));
    }
    open_path(&path)?;
    Ok(path)
}

pub fn list_projects(books: &LocalBooks) -> Result<Vec<String>> {
    let mut stmt = books
        .conn()
        .prepare("SELECT project FROM projects ORDER BY project COLLATE NOCASE")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        let value = row?;
        if !value.trim().is_empty() {
            out.push(value);
        }
    }
    Ok(out)
}

pub fn list_vendors(books: &LocalBooks) -> Result<Vec<VendorRef>> {
    let mut stmt = books
        .conn()
        .prepare("SELECT vendor, gst FROM vendors ORDER BY vendor COLLATE NOCASE")?;
    let rows = stmt.query_map([], |row| {
        Ok(VendorRef {
            vendor: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            gst: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
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

fn like_pattern(query: &str) -> String {
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

pub fn search_office(books: &LocalBooks, query: &str) -> Result<Vec<SearchHit>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let like = like_pattern(q);
    let mut hits = Vec::new();

    {
        let mut stmt = books.conn().prepare(
            "SELECT vendor, gst FROM vendors
             WHERE vendor LIKE ?1 ESCAPE '\\' OR gst LIKE ?1 ESCAPE '\\'
             ORDER BY vendor COLLATE NOCASE LIMIT 20",
        )?;
        let rows = stmt.query_map(params![like], |row| {
            Ok(SearchHit {
                kind: "vendor".into(),
                id: row.get::<_, String>(0)?,
                title: row.get::<_, String>(0)?,
                subtitle: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            })
        })?;
        for row in rows {
            hits.push(row?);
        }
    }
    {
        let mut stmt = books.conn().prepare(
            "SELECT project FROM projects WHERE project LIKE ?1 ESCAPE '\\'
             ORDER BY project COLLATE NOCASE LIMIT 20",
        )?;
        let rows = stmt.query_map(params![like], |row| {
            let project: String = row.get(0)?;
            Ok(SearchHit {
                kind: "project".into(),
                id: project.clone(),
                title: project,
                subtitle: String::new(),
            })
        })?;
        for row in rows {
            hits.push(row?);
        }
    }
    {
        let mut stmt = books.conn().prepare(
            "SELECT id, item_name, type, size FROM inventory
             WHERE item_name LIKE ?1 ESCAPE '\\' OR type LIKE ?1 ESCAPE '\\' OR project LIKE ?1 ESCAPE '\\'
             ORDER BY item_name COLLATE NOCASE LIMIT 20",
        )?;
        let rows = stmt.query_map(params![like], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let ty: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let size: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
            let subtitle = [ty, size]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            Ok(SearchHit {
                kind: "inventory".into(),
                id: id.to_string(),
                title: name,
                subtitle,
            })
        })?;
        for row in rows {
            hits.push(row?);
        }
    }
    {
        let mut stmt = books.conn().prepare(
            "SELECT id, vehicle_number, project, invoice_number FROM logistics
             WHERE vehicle_number LIKE ?1 ESCAPE '\\'
                OR invoice_number LIKE ?1 ESCAPE '\\'
                OR project LIKE ?1 ESCAPE '\\'
             ORDER BY vehicle_number COLLATE NOCASE LIMIT 20",
        )?;
        let rows = stmt.query_map(params![like], |row| {
            let id: i64 = row.get(0)?;
            let vehicle: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
            let project: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let invoice: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
            let subtitle = [project, invoice]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            Ok(SearchHit {
                kind: "logistics".into(),
                id: id.to_string(),
                title: if vehicle.is_empty() {
                    format!("Trip {id}")
                } else {
                    vehicle
                },
                subtitle,
            })
        })?;
        for row in rows {
            hits.push(row?);
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory;
    use crate::vouchers::{apply_voucher_rows, parse_voucher_values};

    fn item(desc: &str, qty: f64, rate: f64, gst: f64) -> PoItemIn {
        PoItemIn {
            id: None,
            item_name: String::new(),
            description: desc.into(),
            qty,
            rate,
            gst_pct: gst,
            amount: 0.0,
        }
    }

    #[test]
    fn calc_po_used_for_sales_total_and_negative_deduction() {
        let preview = preview_po(&[
            item("Cable", 2.0, 50.0, 18.0),
            item("Credit", -1.0, 20.0, 18.0),
        ]);
        assert_eq!(
            crate::core::business_rules::rupees_to_paise(preview.grand_total),
            9440
        );
        let mut books = open_memory().unwrap();
        let saved = save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: "CUST_01".into(),
                po_number: "PO-1".into(),
                client: "CUST_01".into(),
                gst: "GST00DUMMY".into(),
                items: vec![item("Cable", 2.0, 50.0, 18.0)],
            },
        )
        .unwrap();
        assert_eq!(saved.total_value, 118.0);
        assert_eq!(saved.items[0].amount, 118.0);
    }

    #[test]
    fn sales_po_number_unique_per_project() {
        let mut books = open_memory().unwrap();
        save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: "CUST_01".into(),
                po_number: "PO-1".into(),
                client: "".into(),
                gst: "".into(),
                items: vec![],
            },
        )
        .unwrap();
        let err = save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: "CUST_01".into(),
                po_number: "po-1".into(),
                client: "".into(),
                gst: "".into(),
                items: vec![],
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));
        save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: "CUST_02".into(),
                po_number: "PO-1".into(),
                client: "".into(),
                gst: "".into(),
                items: vec![],
            },
        )
        .unwrap();
    }

    #[test]
    fn simple_purchase_skips_items_and_uses_typed_total() {
        let mut books = open_memory().unwrap();
        let saved = save_purchase_po(
            &mut books,
            PurchasePoSave {
                id: None,
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "P-9".into(),
                po_type: "simple".into(),
                total_value: 250.5,
                items: vec![item("ignored", 9.0, 9.0, 18.0)],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(saved.po_type, "simple");
        assert_eq!(saved.total_value, 250.5);
        assert!(saved.items.is_empty());
        assert!(saved.is_dirty);
        assert_eq!(saved.po_number, "P-9");
    }

    #[test]
    fn blank_purchase_allocates_pur_and_pay_classifies() {
        let mut books = open_memory().unwrap();
        let bill = save_purchase_po(
            &mut books,
            PurchasePoSave {
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "".into(),
                po_type: "simple".into(),
                total_value: 100.0,
                goods_received: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(bill.po_number, "PUR-0001");
        let pay = save_purchase_payment(
            &mut books,
            PurchasePaymentSave {
                po_number: bill.po_number.clone(),
                amount_rupees: 40.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pay.pay_number, "PAY-0001");
        assert_eq!(pay.pay_class, "advance");
        assert_eq!(pay.alloc_method, "advance");
        let billed = get_purchase_po(&books, bill.id).unwrap();
        assert_eq!(billed.paid_rupees, 40.0);
        assert_eq!(billed.pay_status, "Partially Paid");
    }

    #[test]
    fn payroll_pf_total_and_unique_month() {
        let books = open_memory().unwrap();
        let person = save_hr_person(
            &books,
            HrPerson {
                id: None,
                name: "Asha".into(),
                role: "Engineer".into(),
                salary: 50000.0,
                ..Default::default()
            },
        )
        .unwrap();
        let pay = save_hr_payroll(
            &books,
            PayrollRow {
                id: None,
                person_id: person.id.unwrap(),
                person_name: String::new(),
                month: "2026-01".into(),
                pf_employee: 1800.0,
                pf_company: 1800.0,
                pf_total: 0.0,
                tds: 500.0,
                total_paid: 47000.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pay.pf_total, 3600.0);
        assert_eq!(pay.salary_number, "SAL-0001");
        let err = save_hr_payroll(
            &books,
            PayrollRow {
                id: None,
                person_id: person.id.unwrap(),
                person_name: String::new(),
                month: "2026-01".into(),
                pf_employee: 1.0,
                pf_company: 1.0,
                pf_total: 0.0,
                tds: 0.0,
                total_paid: 0.0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn voucher_refresh_does_not_delete_office_rows() {
        let mut books = open_memory().unwrap();
        save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: "CUST_01".into(),
                po_number: "PO-KEEP".into(),
                client: "".into(),
                gst: "".into(),
                items: vec![],
            },
        )
        .unwrap();
        save_hr_person(
            &books,
            HrPerson {
                id: None,
                name: "Asha".into(),
                role: "".into(),
                salary: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO inventory (item_name, type, size, quantity, cost, project) VALUES ('kept', 'cable', '10mm', 1, 0, '')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO logistics (project, vehicle_number, invoice_number, start_date, reach_date) VALUES ('CUST_01', 'MH12AB0000', '', '', '')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO documents (name, path, linked_type, linked_id, created_at) VALUES ('keep.pdf', 'C:\\\\keep.pdf', 'project', 'CUST_01', datetime('now'))",
                [],
            )
            .unwrap();
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
        apply_voucher_rows(&mut books, &parse_voucher_values(&[row])).unwrap();
        let sales: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM sales_po", [], |r| r.get(0))
            .unwrap();
        let people: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM hr_people", [], |r| r.get(0))
            .unwrap();
        let inv: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM inventory", [], |r| r.get(0))
            .unwrap();
        let log: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM logistics", [], |r| r.get(0))
            .unwrap();
        let docs: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sales, 1);
        assert_eq!(people, 1);
        assert_eq!(inv, 1);
        assert_eq!(log, 1);
        assert_eq!(docs, 1);
    }

    #[test]
    fn search_hits_vendors_projects_inventory_logistics() {
        let books = open_memory().unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vendors (vendor, gst, updated_at) VALUES ('VEND_02', 'GST00DUMMY', datetime('now'))",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO projects (project, updated_at) VALUES ('CUST_01', datetime('now'))",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO inventory (item_name, type, size, quantity, cost, project) VALUES ('Copper cable', 'cable', '10mm', 4, 10, 'CUST_01')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO logistics (project, vehicle_number, invoice_number, start_date, reach_date) VALUES ('CUST_01', 'MH12AB0000', 'INV-L', '2026-01-01', '')",
                [],
            )
            .unwrap();
        let hits = search_office(&books, "CUST").unwrap();
        assert!(hits.iter().any(|h| h.kind == "project"));
        assert!(hits.iter().any(|h| h.kind == "inventory"));
        assert!(hits.iter().any(|h| h.kind == "logistics"));
        let vendors = search_office(&books, "VEND").unwrap();
        assert!(vendors.iter().any(|h| h.kind == "vendor"));
    }

    #[test]
    fn required_fields_fail_with_real_errors() {
        let mut books = open_memory().unwrap();
        let err = save_sales_po(
            &mut books,
            SalesPoSave {
                id: None,
                project: "  ".into(),
                po_number: "PO-1".into(),
                client: "".into(),
                gst: "".into(),
                items: vec![],
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("Project is required"));
        let err = save_hr_person(
            &books,
            HrPerson {
                id: None,
                name: "".into(),
                role: "".into(),
                salary: 1.0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("Name is required"));
    }
}
