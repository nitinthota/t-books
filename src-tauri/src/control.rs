//! Loopbook Control modes on this PC: Duplicates, Explorer, Rules.
//! Views stay SQLite-only. Secret columns never leave Explorer.

use rusqlite::params;

use crate::db::LocalBooks;
use crate::{BooksError, Result};

const SECRET_RE: &[&str] = &[
    "password",
    "token",
    "secret",
    "private_key",
    "bearer",
    "credential",
];

pub const EXPLORER_TABLES: &[&str] = &[
    "vouchers",
    "voucher_payments",
    "vendors",
    "projects",
    "inventory",
    "logistics",
    "sales_po",
    "sales_po_items",
    "purchase_po",
    "purchase_po_items",
    "purchase_payments",
    "hr_people",
    "hr_payroll",
    "documents",
    "pending_submit",
    "access_cache",
    "rules",
];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSerial {
    pub voucher_no: String,
    pub count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateVariant {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub key: String,
    pub variants: Vec<DuplicateVariant>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesReport {
    pub serials: Vec<DuplicateSerial>,
    pub vendors: Vec<DuplicateGroup>,
    pub projects: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRow {
    pub id: String,
    pub name: String,
    pub when_type: String,
    pub then_action: String,
    pub enabled: bool,
    pub sort_order: i64,
}

fn lookalike_key(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn group_lookalikes(rows: Vec<(String, String)>) -> Vec<DuplicateGroup> {
    let mut map: std::collections::BTreeMap<String, Vec<DuplicateVariant>> =
        std::collections::BTreeMap::new();
    for (id, name) in rows {
        let key = lookalike_key(&name);
        if key.is_empty() {
            continue;
        }
        map.entry(key).or_default().push(DuplicateVariant { id, name });
    }
    map.into_iter()
        .filter(|(_, variants)| {
            let names: std::collections::BTreeSet<_> =
                variants.iter().map(|v| v.name.to_ascii_lowercase()).collect();
            names.len() > 1
        })
        .map(|(key, variants)| DuplicateGroup { key, variants })
        .collect()
}

pub fn list_duplicates(books: &LocalBooks) -> Result<DuplicatesReport> {
    let mut serials = Vec::new();
    {
        let mut stmt = books.conn().prepare(
            "SELECT CAST(voucher_number AS TEXT), COUNT(*) FROM vouchers
             GROUP BY voucher_number HAVING COUNT(*) > 1 ORDER BY COUNT(*) DESC, voucher_number",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DuplicateSerial {
                voucher_no: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        for row in rows {
            serials.push(row?);
        }
    }
    let mut vendors = Vec::new();
    {
        let mut stmt = books.conn().prepare("SELECT vendor, vendor FROM vendors")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            vendors.push(row?);
        }
    }
    let mut projects = Vec::new();
    {
        let mut stmt = books.conn().prepare("SELECT project, project FROM projects")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            projects.push(row?);
        }
    }
    Ok(DuplicatesReport {
        serials,
        vendors: group_lookalikes(vendors),
        projects: group_lookalikes(projects),
    })
}

fn rewrite_vendor(books: &LocalBooks, keep: &str, absorb: &str) -> Result<()> {
    books.conn().execute(
        "UPDATE vouchers SET vendor = ?1 WHERE vendor = ?2 COLLATE NOCASE",
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
        "DELETE FROM vendors WHERE vendor = ?1 COLLATE NOCASE AND vendor != ?2 COLLATE NOCASE",
        params![absorb, keep],
    )?;
    Ok(())
}

fn rewrite_project(books: &LocalBooks, keep: &str, absorb: &str) -> Result<()> {
    books.conn().execute(
        "UPDATE vouchers SET project = ?1 WHERE project = ?2 COLLATE NOCASE",
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

pub fn merge_master(
    books: &LocalBooks,
    kind: &str,
    keep_id: &str,
    absorb_ids: &[String],
) -> Result<i64> {
    let keep = keep_id.trim();
    if keep.is_empty() {
        return Err(BooksError::from("Keep name is required."));
    }
    let absorb: Vec<String> = absorb_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case(keep))
        .collect();
    if absorb.is_empty() {
        return Ok(0);
    }
    match kind {
        "vendor" => {
            for id in &absorb {
                rewrite_vendor(books, keep, id)?;
            }
        }
        "project" => {
            for id in &absorb {
                rewrite_project(books, keep, id)?;
            }
        }
        _ => return Err(BooksError::from("Merge is only for vendor or project names.")),
    }
    Ok(absorb.len() as i64)
}

fn is_secret_column(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_RE.iter().any(|needle| lower.contains(needle))
}

pub fn explore_table(
    books: &LocalBooks,
    table: &str,
) -> Result<Vec<std::collections::BTreeMap<String, String>>> {
    if !EXPLORER_TABLES.contains(&table) {
        return Err(BooksError::from("Unknown table"));
    }
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = books.conn().prepare(&pragma)?;
    let cols_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut cols = Vec::new();
    for col in cols_iter {
        let name = col?;
        if !is_secret_column(&name) {
            cols.push(name);
        }
    }
    if cols.is_empty() {
        return Ok(Vec::new());
    }
    let quoted: Vec<String> = cols.iter().map(|c| format!("\"{c}\"")).collect();
    let sql = format!("SELECT {} FROM {table} LIMIT 200", quoted.join(", "));
    let mut stmt = books.conn().prepare(&sql)?;
    let col_count = cols.len();
    let rows = stmt.query_map([], |row| {
        let mut map = std::collections::BTreeMap::new();
        for (i, name) in cols.iter().enumerate().take(col_count) {
            let value: Option<String> = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => None,
                rusqlite::types::ValueRef::Integer(n) => Some(n.to_string()),
                rusqlite::types::ValueRef::Real(n) => Some(n.to_string()),
                rusqlite::types::ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
                rusqlite::types::ValueRef::Blob(_) => Some("[blob]".into()),
            };
            map.insert(name.clone(), value.unwrap_or_default());
        }
        Ok(map)
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn list_rules(books: &LocalBooks) -> Result<Vec<RuleRow>> {
    let mut stmt = books.conn().prepare(
        "SELECT id, name, when_type, then_action, enabled, sort_order FROM rules ORDER BY sort_order, id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RuleRow {
            id: row.get(0)?,
            name: row.get(1)?,
            when_type: row.get(2)?,
            then_action: row.get(3)?,
            enabled: row.get::<_, i64>(4)? != 0,
            sort_order: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn toggle_rule(books: &LocalBooks, id: &str, enabled: bool) -> Result<RuleRow> {
    let n = books.conn().execute(
        "UPDATE rules SET enabled = ?1 WHERE id = ?2",
        params![if enabled { 1 } else { 0 }, id],
    )?;
    if n == 0 {
        return Err(BooksError::from("That rule is not on this PC."));
    }
    list_rules(books)?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| BooksError::from("That rule is not on this PC."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory;

    #[test]
    fn explorer_hides_password_hash() {
        let books = open_memory().unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO users_local (email, password_hash, created_at) VALUES ('a@b.c', 'x', datetime('now'))",
                [],
            )
            .unwrap();
        let err = explore_table(&books, "users_local").unwrap_err();
        assert!(err.to_string().contains("Unknown table"));
        let rows = explore_table(&books, "access_cache").unwrap();
        if let Some(row) = rows.first() {
            for key in row.keys() {
                assert!(!key.to_ascii_lowercase().contains("password"));
            }
        }
    }

    #[test]
    fn lookalike_vendors_merge_keeps_selected_name() {
        let books = open_memory().unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vendors (vendor) VALUES ('VEND 02'), ('VEND-02')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vouchers (voucher_number, vendor) VALUES (1001, 'VEND-02')",
                [],
            )
            .unwrap();
        let report = list_duplicates(&books).unwrap();
        assert!(report.vendors.iter().any(|g| g.variants.len() >= 2));
        let merged = merge_master(
            &books,
            "vendor",
            "VEND 02",
            &["VEND-02".into()],
        )
        .unwrap();
        assert_eq!(merged, 1);
        let vendor: String = books
            .conn()
            .query_row(
                "SELECT vendor FROM vouchers WHERE voucher_number = 1001",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vendor, "VEND 02");
        let serials = list_duplicates(&books).unwrap().serials;
        assert!(serials.is_empty());
    }

    #[test]
    fn rules_seed_and_toggle() {
        let books = open_memory().unwrap();
        let rules = list_rules(&books).unwrap();
        assert!(rules.len() >= 6);
        let first = &rules[0];
        let toggled = toggle_rule(&books, &first.id, false).unwrap();
        assert!(!toggled.enabled);
    }
}
