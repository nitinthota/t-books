//! Park a deleted row for 30 days. Live lists hide it. Restore puts it back.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::{data_dir, LocalBooks};
use crate::{BooksError, Result};

pub const HOLD_DAYS: i64 = 30;

fn wipe_hive(books: &LocalBooks, kind: &str, key: &str) {
    let (hive_kind, hive_key) = crate::hive_erase::kind_for_parked(kind, key);
    crate::hive_erase::erase_hive_row_quiet(books, hive_kind, &hive_key);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedRow {
    pub id: i64,
    pub kind: String,
    pub key: String,
    pub label: String,
    pub deleted_at: String,
    pub purge_after: String,
    pub days_left: i64,
}

fn ensure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS archived_rows (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          kind TEXT NOT NULL,
          key TEXT NOT NULL,
          label TEXT NOT NULL DEFAULT '',
          payload TEXT NOT NULL DEFAULT '{}',
          file_name TEXT,
          deleted_at TEXT NOT NULL,
          purge_after TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_archived_kind_key ON archived_rows(kind, key);
        "#,
    )?;
    Ok(())
}

fn archive_dir() -> std::path::PathBuf {
    let dir = data_dir().join("archive");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn now_parts(conn: &Connection) -> Result<(String, String)> {
    let deleted_at: String = conn.query_row("SELECT datetime('now')", [], |row| row.get(0))?;
    let purge_after: String =
        conn.query_row("SELECT datetime('now', '+30 days')", [], |row| row.get(0))?;
    Ok((deleted_at, purge_after))
}

fn maps(conn: &Connection, sql: &str, p: impl rusqlite::Params) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
    let mut rows = stmt.query(p)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let mut obj = serde_json::Map::new();
        for (i, name) in names.iter().enumerate() {
            let val = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => Value::Null,
                rusqlite::types::ValueRef::Integer(n) => json!(n),
                rusqlite::types::ValueRef::Real(n) => json!(n),
                rusqlite::types::ValueRef::Text(t) => json!(String::from_utf8_lossy(t).to_string()),
                rusqlite::types::ValueRef::Blob(_) => Value::Null,
            };
            obj.insert(name.clone(), val);
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

fn write_file(kind: &str, key: &str, payload: &Value) -> Option<String> {
    let safe: String = format!("{kind}_{key}")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let name = format!("{safe}.json");
    let path = archive_dir().join(&name);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".into()),
    )
    .ok()?;
    Some(name)
}

fn park_payload(books: &LocalBooks, kind: &str, key: &str, label: &str, payload: Value) -> Result<i64> {
    ensure(books.conn())?;
    let (deleted_at, purge_after) = now_parts(books.conn())?;
    let file_name = write_file(kind, key, &payload);
    books.conn().execute(
        "INSERT INTO archived_rows (kind, key, label, payload, file_name, deleted_at, purge_after)\n         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![kind, key, label, payload.to_string(), file_name, deleted_at, purge_after],
    )?;
    Ok(books.conn().last_insert_rowid())
}

pub fn is_parked(books: &LocalBooks, kind: &str, key: &str) -> bool {
    let _ = ensure(books.conn());
    books
        .conn()
        .query_row(
            "SELECT 1 FROM archived_rows WHERE kind = ?1 AND key = ?2 AND purge_after >= datetime('now') LIMIT 1",
            params![kind, key],
            |_| Ok(1i64),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
}

pub fn park_voucher(books: &LocalBooks, number: i64) -> Result<()> {
    ensure(books.conn())?;
    let head = maps(books.conn(), "SELECT * FROM vouchers WHERE voucher_number = ?1", params![number])?;
    if head.is_empty() {
        return Err("That voucher was not found on this computer.".into());
    }
    let pays = maps(
        books.conn(),
        "SELECT * FROM voucher_payments WHERE voucher_number = ?1",
        params![number],
    )?;
    let vendor = head[0].get("vendor").and_then(|v| v.as_str()).unwrap_or("").to_string();
    park_payload(
        books,
        "voucher",
        &number.to_string(),
        &format!("Voucher {number} {vendor}"),
        json!({ "table": "vouchers", "rows": head, "children": [{ "table": "voucher_payments", "rows": pays }] }),
    )?;
    books.conn().execute("DELETE FROM voucher_payments WHERE voucher_number = ?1", params![number])?;
    books.conn().execute("DELETE FROM vouchers WHERE voucher_number = ?1", params![number])?;
    let _ = books.conn().execute(
        "DELETE FROM pending_submit WHERE kind = 'voucher' AND key = ?1",
        params![number.to_string()],
    );
    wipe_hive(books, "voucher", &number.to_string());
    Ok(())
}

pub fn park_purchase(books: &LocalBooks, id: i64) -> Result<()> {
    ensure(books.conn())?;
    let head = maps(books.conn(), "SELECT * FROM purchase_po WHERE id = ?1", params![id])?;
    if head.is_empty() {
        return Err("That purchase bill was not found on this computer.".into());
    }
    let items = maps(books.conn(), "SELECT * FROM purchase_po_items WHERE po_id = ?1", params![id])?;
    let po = head[0].get("po_number").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let pays = maps(books.conn(), "SELECT * FROM purchase_payments WHERE po_number = ?1", params![&po])?;
    let vendor = head[0].get("vendor").and_then(|v| v.as_str()).unwrap_or("").to_string();
    park_payload(
        books,
        "purchase",
        &po,
        &format!("{po} {vendor}"),
        json!({
            "table": "purchase_po",
            "rows": head,
            "children": [
                { "table": "purchase_po_items", "rows": items },
                { "table": "purchase_payments", "rows": pays }
            ]
        }),
    )?;
    books.conn().execute("DELETE FROM purchase_payments WHERE po_number = ?1", params![&po])?;
    books.conn().execute("DELETE FROM purchase_po_items WHERE po_id = ?1", params![id])?;
    books.conn().execute("DELETE FROM purchase_po WHERE id = ?1", params![id])?;
    wipe_hive(books, "purchase", &po);
    for pay in &pays {
        if let Some(pay_no) = pay.get("pay_number").and_then(|v| v.as_str()) {
            wipe_hive(books, "payment", pay_no);
        }
    }
    Ok(())
}

pub fn park_sales(books: &LocalBooks, id: i64) -> Result<()> {
    ensure(books.conn())?;
    let head = maps(books.conn(), "SELECT * FROM sales_po WHERE id = ?1", params![id])?;
    if head.is_empty() {
        return Err("That sales order was not found on this computer.".into());
    }
    let items = maps(books.conn(), "SELECT * FROM sales_po_items WHERE po_id = ?1", params![id])?;
    let po = head[0].get("po_number").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let job = head[0].get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
    park_payload(
        books,
        "sales_po",
        &format!("{po}@{job}"),
        &format!("{po} {job}"),
        json!({ "table": "sales_po", "rows": head, "children": [{ "table": "sales_po_items", "rows": items }] }),
    )?;
    books.conn().execute("DELETE FROM sales_po_items WHERE po_id = ?1", params![id])?;
    books.conn().execute("DELETE FROM sales_po WHERE id = ?1", params![id])?;
    wipe_hive(books, "sales_po", &format!("{po}@{job}"));
    Ok(())
}

pub fn park_payment(books: &LocalBooks, id: i64) -> Result<()> {
    ensure(books.conn())?;
    let head = maps(books.conn(), "SELECT * FROM purchase_payments WHERE id = ?1", params![id])?;
    if head.is_empty() {
        return Err("That payment was not found on this computer.".into());
    }
    let key = head[0].get("pay_number").and_then(|v| v.as_str()).unwrap_or("").to_string();
    park_payload(
        books,
        "payment",
        &key,
        &key,
        json!({ "table": "purchase_payments", "rows": head, "children": [] }),
    )?;
    books.conn().execute("DELETE FROM purchase_payments WHERE id = ?1", params![id])?;
    wipe_hive(books, "payment", &key);
    Ok(())
}

pub fn park_simple(books: &LocalBooks, kind: &str, table: &str, id: i64) -> Result<()> {
    ensure(books.conn())?;
    let head = maps(books.conn(), &format!("SELECT * FROM {table} WHERE id = ?1"), params![id])?;
    if head.is_empty() {
        return Err("That row was not found on this computer.".into());
    }
    let label = head[0]
        .get("name")
        .or_else(|| head[0].get("item_name"))
        .or_else(|| head[0].get("vehicle_number"))
        .or_else(|| head[0].get("salary_number"))
        .and_then(|v| v.as_str())
        .unwrap_or(kind)
        .to_string();
    park_payload(
        books,
        kind,
        &id.to_string(),
        &label,
        json!({ "table": table, "rows": head, "children": [] }),
    )?;
    if table == "hr_people" {
        let pays = maps(books.conn(), "SELECT * FROM hr_payroll WHERE person_id = ?1", params![id])?;
        if let Some(id_row) = books
            .conn()
            .query_row(
                "SELECT id FROM archived_rows WHERE kind = ?1 AND key = ?2 ORDER BY id DESC LIMIT 1",
                params![kind, id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            let payload = json!({ "table": table, "rows": head, "children": [{ "table": "hr_payroll", "rows": pays }] });
            books.conn().execute(
                "UPDATE archived_rows SET payload = ?1 WHERE id = ?2",
                params![payload.to_string(), id_row],
            )?;
        }
        books.conn().execute("DELETE FROM hr_payroll WHERE person_id = ?1", params![id])?;
    }
    if table == "documents" {
        if let Some(path) = head[0].get("path").and_then(|v| v.as_str()) {
            if !path.is_empty() {
                let src = std::path::PathBuf::from(path);
                if src.exists() {
                    let dest = archive_dir().join(src.file_name().unwrap_or_default());
                    let _ = std::fs::rename(&src, dest);
                }
            }
        }
    }
    books.conn().execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id])?;
    wipe_hive(books, kind, &id.to_string());
    Ok(())
}

pub fn list_archive(books: &LocalBooks) -> Result<Vec<ArchivedRow>> {
    ensure(books.conn())?;
    let _ = purge_expired(books);
    let mut stmt = books.conn().prepare(
        "SELECT id, kind, key, label, deleted_at, purge_after,\n                CAST(julianday(purge_after) - julianday('now') AS INTEGER)\n         FROM archived_rows\n         WHERE purge_after >= datetime('now')\n         ORDER BY deleted_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ArchivedRow {
            id: row.get(0)?,
            kind: row.get(1)?,
            key: row.get(2)?,
            label: row.get(3)?,
            deleted_at: row.get(4)?,
            purge_after: row.get(5)?,
            days_left: row.get::<_, i64>(6)?.max(0),
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn insert_maps(conn: &Connection, table: &str, rows: &[Value]) -> Result<()> {
    for row in rows {
        let obj = match row.as_object() {
            Some(o) => o,
            None => continue,
        };
        let cols: Vec<String> = obj.keys().cloned().collect();
        if cols.is_empty() {
            continue;
        }
        let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT OR REPLACE INTO {table} ({}) VALUES ({})",
            cols.join(", "),
            placeholders.join(", ")
        );
        let values: Vec<String> = cols
            .iter()
            .map(|c| match obj.get(c) {
                Some(Value::Null) | None => String::new(),
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string().trim_matches('"').to_string(),
            })
            .collect();
        conn.execute(&sql, rusqlite::params_from_iter(values.iter()))?;
    }
    Ok(())
}

pub fn restore_archive(books: &LocalBooks, id: i64) -> Result<ArchivedRow> {
    ensure(books.conn())?;
    let payload: String = books
        .conn()
        .query_row(
            "SELECT payload FROM archived_rows WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|_| BooksError::from("That removed row was not found."))?;
    let parsed: Value = serde_json::from_str(&payload).unwrap_or(json!({}));
    if let Some(table) = parsed.get("table").and_then(|v| v.as_str()) {
        if let Some(rows) = parsed.get("rows").and_then(|v| v.as_array()) {
            insert_maps(books.conn(), table, rows)?;
        }
    }
    if let Some(children) = parsed.get("children").and_then(|v| v.as_array()) {
        for child in children {
            if let (Some(table), Some(rows)) = (
                child.get("table").and_then(|v| v.as_str()),
                child.get("rows").and_then(|v| v.as_array()),
            ) {
                insert_maps(books.conn(), table, rows)?;
            }
        }
    }
    let listed = list_archive(books)?
        .into_iter()
        .find(|r| r.id == id)
        .unwrap_or(ArchivedRow {
            id,
            kind: String::new(),
            key: String::new(),
            label: String::new(),
            deleted_at: String::new(),
            purge_after: String::new(),
            days_left: 0,
        });
    books.conn().execute("DELETE FROM archived_rows WHERE id = ?1", params![id])?;
    Ok(listed)
}

pub fn purge_expired(books: &LocalBooks) -> Result<u32> {
    ensure(books.conn())?;
    let names: Vec<String> = {
        let mut stmt = books.conn().prepare(
            "SELECT file_name FROM archived_rows WHERE purge_after < datetime('now') AND file_name IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
        let mut out = Vec::new();
        for row in rows {
            if let Some(name) = row? {
                out.push(name);
            }
        }
        out
    };
    for name in names {
        let _ = std::fs::remove_file(archive_dir().join(name));
    }
    let n = books
        .conn()
        .execute("DELETE FROM archived_rows WHERE purge_after < datetime('now')", [])?;
    Ok(n as u32)
}

/// After a hive Refresh, drop live copies that are still parked.
pub fn keep_parked_out(books: &LocalBooks) -> Result<()> {
    ensure(books.conn())?;
    let rows = list_archive(books)?;
    for row in rows {
        match row.kind.as_str() {
            "voucher" => {
                let _ = books.conn().execute(
                    "DELETE FROM voucher_payments WHERE CAST(voucher_number AS TEXT) = ?1",
                    params![row.key],
                );
                let _ = books.conn().execute(
                    "DELETE FROM vouchers WHERE CAST(voucher_number AS TEXT) = ?1",
                    params![row.key],
                );
            }
            "purchase" => {
                let _ = books
                    .conn()
                    .execute("DELETE FROM purchase_po WHERE po_number = ?1", params![row.key]);
            }
            "sales_po" => {
                if let Some((po, job)) = row.key.split_once('@') {
                    let _ = books.conn().execute(
                        "DELETE FROM sales_po WHERE po_number = ?1 AND project = ?2",
                        params![po, job],
                    );
                }
            }
            "payment" => {
                let _ = books.conn().execute(
                    "DELETE FROM purchase_payments WHERE pay_number = ?1",
                    params![row.key],
                );
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    #[test]
    fn park_and_restore_purchase() {
        let books = open_memory().unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO purchase_po (id, project, vendor, po_number, type, total_value) VALUES (1, 'JOB-A', 'VEND_02', 'PUR-0001', 'contract', 100)",
                [],
            )
            .unwrap();
        park_purchase(&books, 1).unwrap();
        let live: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM purchase_po", [], |r| r.get(0))
            .unwrap();
        assert_eq!(live, 0);
        let list = list_archive(&books).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].key, "PUR-0001");
        restore_archive(&books, list[0].id).unwrap();
        let live: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM purchase_po WHERE po_number = 'PUR-0001'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(live, 1);
        assert!(list_archive(&books).unwrap().is_empty());
    }
}
