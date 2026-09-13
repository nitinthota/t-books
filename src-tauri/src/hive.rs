//! Hive contract: Google Sheets as a clumsy central ledger.
//! Views never call this. Submit is one row, CAS on fp+rev. No LWW on money.

use rusqlite::{params, OptionalExtension};

use crate::db::LocalBooks;
use crate::{BooksError, Result};

pub const KIND_VOUCHER: &str = "voucher";
pub const KIND_PURCHASE: &str = "purchase";
pub const KIND_PAYMENT: &str = "payment";
pub const KIND_SALARY: &str = "salary";
pub const KIND_SALES_PO: &str = "sales_po";
#[allow(dead_code)]
pub const KIND_INVENTORY: &str = "inventory";
#[allow(dead_code)]
pub const KIND_LOGISTICS: &str = "logistics";
#[allow(dead_code)]
pub const KIND_DOCUMENT: &str = "document";
pub const KIND_ACCESS: &str = "access";

pub const ACCESS_HEADERS: &[&str] = &["Name", "Email", "Role", "Active"];
pub const VOUCHER_HEADERS: &[&str] = &[
    "serial_no",
    "voucher_date",
    "tax_inv_no_date",
    "vendor_name",
    "bank_name",
    "account_no",
    "ifsc",
    "gst_no",
    "project_name",
];

/// Windows hive kinds. Same set as Loopbook modules that leave this PC on Submit.
pub const HIVE_KINDS: &[&str] = &[
    KIND_ACCESS,
    KIND_VOUCHER,
    KIND_PURCHASE,
    KIND_PAYMENT,
    KIND_SALARY,
    KIND_SALES_PO,
    KIND_INVENTORY,
    KIND_LOGISTICS,
    KIND_DOCUMENT,
];

/// `{KEY} was just updated by another user. Reload and submit again.`
pub fn hive_conflict_message(key: &str) -> String {
    format!("{key} was just updated by another user. Reload and submit again.")
}

pub fn sales_po_hive_key(po_number: &str, project: &str) -> String {
    format!("{}@{}", po_number.trim(), project.trim())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirtyKey {
    pub kind: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiveRow {
    pub key: String,
    pub kind: String,
    pub fp: String,
    pub rev: i64,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CasOutcome {
    Ok {
        key: String,
        fp: String,
        rev: i64,
    },
    Conflict {
        key: String,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureTab {
    Exists,
    BootstrappedHeaders,
}

/// Fake hive for tests. No live sheet in CI.
pub trait Hive {
    fn get_row(&self, kind: &str, key: &str) -> Result<Option<HiveRow>>;
    fn cas_row(
        &mut self,
        kind: &str,
        key: &str,
        base_fp: &str,
        base_rev: i64,
        cells: &[String],
        new_fp: &str,
    ) -> Result<CasOutcome>;
    fn list_keys(&self, kind: &str) -> Result<Vec<String>>;
    fn ensure_tab(&mut self, kind: &str, headers: &[String]) -> Result<EnsureTab>;
}

#[derive(Debug, Clone)]
pub struct MemoryHive {
    /// kind → (optional header row, data rows)
    tabs: std::collections::BTreeMap<String, Tab>,
    pub fail_writes: bool,
}

#[derive(Debug, Clone)]
struct Tab {
    #[allow(dead_code)]
    headers: Vec<String>,
    rows: Vec<HiveRow>,
}

impl MemoryHive {
    pub fn new() -> Self {
        Self {
            tabs: std::collections::BTreeMap::new(),
            fail_writes: false,
        }
    }

    pub fn with_tab(kind: &str, headers: &[&str]) -> Self {
        let mut hive = Self::new();
        let _ = hive.ensure_tab(
            kind,
            &headers.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        );
        hive
    }
}

impl Default for MemoryHive {
    fn default() -> Self {
        Self::new()
    }
}

impl Hive for MemoryHive {
    fn get_row(&self, kind: &str, key: &str) -> Result<Option<HiveRow>> {
        let Some(tab) = self.tabs.get(kind) else {
            return Ok(None);
        };
        Ok(tab.rows.iter().find(|r| r.key == key).cloned())
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
        if self.fail_writes {
            return Err(BooksError::from("Could not reach Google Sheets from this PC."));
        }
        if !self.tabs.contains_key(kind) {
            return Err(BooksError::from(format!(
                "Hive tab {kind} is missing. Owner can bootstrap headers only."
            )));
        }
        let tab = self.tabs.get_mut(kind).expect("tab");
        let existing = tab.rows.iter().position(|r| r.key == key);
        match existing {
            None => {
                if base_rev != 0 && !base_fp.is_empty() {
                    return Ok(CasOutcome::Conflict {
                        key: key.to_string(),
                        message: hive_conflict_message(key),
                    });
                }
                tab.rows.push(HiveRow {
                    key: key.to_string(),
                    kind: kind.to_string(),
                    fp: new_fp.to_string(),
                    rev: 1,
                    cells: cells.to_vec(),
                });
                Ok(CasOutcome::Ok {
                    key: key.to_string(),
                    fp: new_fp.to_string(),
                    rev: 1,
                })
            }
            Some(idx) => {
                let row = &tab.rows[idx];
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
                tab.rows[idx] = HiveRow {
                    key: key.to_string(),
                    kind: kind.to_string(),
                    fp: new_fp.to_string(),
                    rev: base_rev + 1,
                    cells: cells.to_vec(),
                };
                Ok(CasOutcome::Ok {
                    key: key.to_string(),
                    fp: new_fp.to_string(),
                    rev: base_rev + 1,
                })
            }
        }
    }

    fn list_keys(&self, kind: &str) -> Result<Vec<String>> {
        let Some(tab) = self.tabs.get(kind) else {
            return Ok(Vec::new());
        };
        Ok(tab.rows.iter().map(|r| r.key.clone()).collect())
    }

    fn ensure_tab(&mut self, kind: &str, headers: &[String]) -> Result<EnsureTab> {
        if self.tabs.contains_key(kind) {
            return Ok(EnsureTab::Exists);
        }
        self.tabs.insert(
            kind.to_string(),
            Tab {
                headers: headers.to_vec(),
                rows: Vec::new(),
            },
        );
        Ok(EnsureTab::BootstrappedHeaders)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSubmit {
    pub key: String,
    pub kind: String,
    pub payload: String,
    pub base_fp: String,
    pub base_rev: i64,
    pub attempts: i64,
    pub last_error: String,
}

pub fn enqueue_submit(
    books: &LocalBooks,
    key: &str,
    kind: &str,
    payload: &str,
    base_fp: &str,
    base_rev: i64,
) -> Result<()> {
    books.conn().execute(
        r#"
        INSERT INTO pending_submit (key, kind, payload, base_fp, base_rev, attempts, last_error, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 0, '', datetime('now'), datetime('now'))
        ON CONFLICT(key, kind) DO UPDATE SET
          payload = excluded.payload,
          base_fp = excluded.base_fp,
          base_rev = excluded.base_rev,
          updated_at = datetime('now')
        "#,
        params![key, kind, payload, base_fp, base_rev],
    )?;
    Ok(())
}

pub fn list_pending(books: &LocalBooks) -> Result<Vec<PendingSubmit>> {
    let mut stmt = books.conn().prepare(
        "SELECT key, kind, payload, base_fp, base_rev, attempts, COALESCE(last_error,'')
         FROM pending_submit ORDER BY kind, key",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PendingSubmit {
            key: row.get(0)?,
            kind: row.get(1)?,
            payload: row.get(2)?,
            base_fp: row.get(3)?,
            base_rev: row.get(4)?,
            attempts: row.get(5)?,
            last_error: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn clear_pending(books: &LocalBooks, key: &str, kind: &str) -> Result<()> {
    books.conn().execute(
        "DELETE FROM pending_submit WHERE key = ?1 AND kind = ?2",
        params![key, kind],
    )?;
    Ok(())
}

pub fn bump_pending_attempt(books: &LocalBooks, key: &str, kind: &str, error: &str) -> Result<()> {
    let safe = if error.to_ascii_lowercase().contains("password")
        || error.to_ascii_lowercase().contains("private_key")
        || error.to_ascii_lowercase().contains("bearer ")
    {
        "Google write failed."
    } else {
        error
    };
    books.conn().execute(
        "UPDATE pending_submit SET attempts = attempts + 1, last_error = ?3, updated_at = datetime('now')
         WHERE key = ?1 AND kind = ?2",
        params![key, kind, safe],
    )?;
    Ok(())
}

pub fn list_dirty_keys(books: &LocalBooks) -> Result<Vec<DirtyKey>> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut stmt = books.conn().prepare(
        "SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1 ORDER BY voucher_number",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    for row in rows {
        let n = row?;
        let key = n.to_string();
        if seen.insert((KIND_VOUCHER.to_string(), key.clone())) {
            out.push(DirtyKey {
                kind: KIND_VOUCHER.into(),
                key,
            });
        }
    }
    for pending in list_pending(books)? {
        if seen.insert((pending.kind.clone(), pending.key.clone())) {
            out.push(DirtyKey {
                kind: pending.kind,
                key: pending.key,
            });
        }
    }
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_PURCHASE,
        "SELECT po_number FROM purchase_po WHERE COALESCE(is_dirty,0) = 1 AND TRIM(po_number) != ''",
    )?;
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_PAYMENT,
        "SELECT pay_number FROM purchase_payments WHERE COALESCE(is_dirty,0) = 1 AND TRIM(pay_number) != ''",
    )?;
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_SALARY,
        "SELECT salary_number FROM hr_payroll WHERE COALESCE(is_dirty,0) = 1 AND TRIM(COALESCE(salary_number,'')) != ''",
    )?;
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_SALES_PO,
        "SELECT TRIM(po_number) || '@' || TRIM(project) FROM sales_po WHERE COALESCE(is_dirty,0) = 1 AND TRIM(po_number) != ''",
    )?;
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_INVENTORY,
        "SELECT 'INV-' || id FROM inventory WHERE COALESCE(is_dirty,0) = 1",
    )?;
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_LOGISTICS,
        "SELECT 'TRIP-' || id FROM logistics WHERE COALESCE(is_dirty,0) = 1",
    )?;
    push_dirty_sql(
        books,
        &mut out,
        &mut seen,
        KIND_DOCUMENT,
        "SELECT 'DOC-' || id FROM documents WHERE COALESCE(is_dirty,0) = 1",
    )?;
    Ok(out)
}

fn push_dirty_sql(
    books: &LocalBooks,
    out: &mut Vec<DirtyKey>,
    seen: &mut std::collections::BTreeSet<(String, String)>,
    kind: &str,
    sql: &str,
) -> Result<()> {
    let mut stmt = books.conn().prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for row in rows {
        let key = row?;
        if seen.insert((kind.to_string(), key.clone())) {
            out.push(DirtyKey {
                kind: kind.into(),
                key,
            });
        }
    }
    Ok(())
}

pub fn pending_payload_json(kind: &str, key: &str) -> String {
    serde_json::json!({ "kind": kind, "key": key }).to_string()
}

pub fn tab_name(kind: &str) -> &'static str {
    match kind {
        KIND_ACCESS => "Access",
        KIND_VOUCHER => "Voucher_Raw_Data",
        KIND_PURCHASE => "Purchase",
        KIND_PAYMENT => "Payments",
        KIND_SALARY => "Payroll",
        KIND_SALES_PO => "Sales_PO",
        KIND_INVENTORY => "Inventory",
        KIND_LOGISTICS => "Logistics",
        KIND_DOCUMENT => "Documents",
        _ => "Hive",
    }
}

pub fn tab_headers(kind: &str) -> Vec<String> {
    match kind {
        KIND_ACCESS => ACCESS_HEADERS.iter().map(|s| (*s).to_string()).collect(),
        KIND_VOUCHER => VOUCHER_HEADERS.iter().map(|s| (*s).to_string()).collect(),
        KIND_PURCHASE => vec![
            "po_number".into(),
            "project".into(),
            "vendor".into(),
            "type".into(),
            "goods_received".into(),
            "tax_invoice_no".into(),
            "tax_invoice_date".into(),
            "total_value".into(),
            "fp".into(),
            "rev".into(),
        ],
        KIND_PAYMENT => vec![
            "pay_number".into(),
            "po_number".into(),
            "vendor".into(),
            "amount".into(),
            "alloc_method".into(),
            "pay_class".into(),
            "pay_date".into(),
            "fp".into(),
            "rev".into(),
        ],
        KIND_SALARY => vec![
            "salary_number".into(),
            "person".into(),
            "month".into(),
            "kind".into(),
            "net".into(),
            "fp".into(),
            "rev".into(),
        ],
        KIND_SALES_PO => vec![
            "key".into(),
            "po_number".into(),
            "project".into(),
            "client".into(),
            "gst".into(),
            "total_value".into(),
            "fp".into(),
            "rev".into(),
        ],
        KIND_INVENTORY => vec![
            "key".into(),
            "item_name".into(),
            "type".into(),
            "size".into(),
            "qty".into(),
            "cost".into(),
            "project".into(),
            "fp".into(),
            "rev".into(),
        ],
        KIND_LOGISTICS => vec![
            "key".into(),
            "project".into(),
            "vehicle".into(),
            "invoice".into(),
            "start".into(),
            "reach".into(),
            "fp".into(),
            "rev".into(),
        ],
        KIND_DOCUMENT => vec![
            "key".into(),
            "name".into(),
            "linked_type".into(),
            "linked_id".into(),
            "fp".into(),
            "rev".into(),
        ],
        _ => vec!["key".into(), "fp".into(), "rev".into()],
    }
}

/// Submit one hive row: enqueue outbox, CAS, then clear on success. Never silent overwrite.
pub fn submit_hive_row(
    books: &LocalBooks,
    hive: &mut dyn Hive,
    kind: &str,
    key: &str,
    base_fp: &str,
    base_rev: i64,
    cells: &[String],
    new_fp: &str,
    payload: &str,
) -> Result<CasOutcome> {
    enqueue_submit(books, key, kind, payload, base_fp, base_rev)?;
    match hive.cas_row(kind, key, base_fp, base_rev, cells, new_fp) {
        Ok(CasOutcome::Ok { key, fp, rev }) => {
            if let Err(err) = clear_pending(books, &key, kind) {
                crate::log::event(
                    crate::log::Level::Error,
                    "submit",
                    "write",
                    None,
                    "local_update_failed",
                );
                return Err(err);
            }
            Ok(CasOutcome::Ok { key, fp, rev })
        }
        Ok(conflict @ CasOutcome::Conflict { .. }) => Ok(conflict),
        Err(err) => {
            let _ = bump_pending_attempt(books, key, kind, &err.to_string());
            Err(err)
        }
    }
}

pub fn get_hive_rev(books: &LocalBooks, voucher_number: i64) -> Result<i64> {
    let rev: Option<i64> = books
        .conn()
        .query_row(
            "SELECT COALESCE(hive_rev, 0) FROM vouchers WHERE voucher_number = ?1",
            params![voucher_number],
            |row| row.get(0),
        )
        .optional()?;
    Ok(rev.unwrap_or(0))
}

#[allow(dead_code)]
pub fn set_hive_rev(books: &LocalBooks, voucher_number: i64, rev: i64) -> Result<()> {
    books.conn().execute(
        "UPDATE vouchers SET hive_rev = ?1 WHERE voucher_number = ?2",
        params![rev, voucher_number],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory;

    #[test]
    fn cas_matches_fp_and_rev() {
        let mut hive = MemoryHive::with_tab(KIND_VOUCHER, ACCESS_HEADERS);
        let first = hive
            .cas_row(KIND_VOUCHER, "20", "", 0, &["20".into()], "aaa")
            .unwrap();
        assert_eq!(
            first,
            CasOutcome::Ok {
                key: "20".into(),
                fp: "aaa".into(),
                rev: 1
            }
        );
        let second = hive
            .cas_row(KIND_VOUCHER, "20", "aaa", 1, &["20".into(), "x".into()], "bbb")
            .unwrap();
        assert_eq!(
            second,
            CasOutcome::Ok {
                key: "20".into(),
                fp: "bbb".into(),
                rev: 2
            }
        );
        let conflict = hive
            .cas_row(KIND_VOUCHER, "20", "stale", 1, &["20".into()], "ccc")
            .unwrap();
        match conflict {
            CasOutcome::Conflict { key, message } => {
                assert_eq!(key, "20");
                assert_eq!(message, hive_conflict_message("20"));
                assert!(message.contains("just"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(hive.get_row(KIND_VOUCHER, "20").unwrap().unwrap().fp, "bbb");
    }

    #[test]
    fn different_keys_never_block() {
        let mut hive = MemoryHive::with_tab(KIND_VOUCHER, &["serial_no"]);
        hive.cas_row(KIND_VOUCHER, "20", "", 0, &["20".into()], "a").unwrap();
        hive.cas_row(KIND_PURCHASE, "PUR-0001", "", 0, &["PUR-0001".into()], "b")
            .unwrap_err();
        let _ = hive.ensure_tab(KIND_PURCHASE, &tab_headers(KIND_PURCHASE));
        let pay = hive
            .cas_row(KIND_PURCHASE, "PUR-0001", "", 0, &["PUR-0001".into()], "b")
            .unwrap();
        assert!(matches!(pay, CasOutcome::Ok { .. }));
        let keys = hive.list_keys(KIND_VOUCHER).unwrap();
        assert_eq!(keys, vec!["20"]);
    }

    #[test]
    fn missing_tab_bootstraps_headers_only() {
        let mut hive = MemoryHive::new();
        assert!(hive.get_row(KIND_ACCESS, "x").unwrap().is_none());
        let created = hive
            .ensure_tab(KIND_ACCESS, &tab_headers(KIND_ACCESS))
            .unwrap();
        assert_eq!(created, EnsureTab::BootstrappedHeaders);
        assert!(hive.list_keys(KIND_ACCESS).unwrap().is_empty());
        let again = hive
            .ensure_tab(KIND_ACCESS, &tab_headers(KIND_ACCESS))
            .unwrap();
        assert_eq!(again, EnsureTab::Exists);
    }

    #[test]
    fn outbox_survives_failed_write_and_clears_on_success() {
        let books = open_memory().unwrap();
        let mut hive = MemoryHive::with_tab(KIND_VOUCHER, &["serial_no"]);
        hive.fail_writes = true;
        let err = submit_hive_row(
            &books,
            &mut hive,
            KIND_VOUCHER,
            "1001",
            "",
            0,
            &["1001".into()],
            "fp1",
            "{}",
        )
        .unwrap_err();
        assert!(!err.to_string().is_empty());
        let pending = list_pending(&books).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].key, "1001");
        assert_eq!(pending[0].attempts, 1);
        hive.fail_writes = false;
        let ok = submit_hive_row(
            &books,
            &mut hive,
            KIND_VOUCHER,
            "1001",
            "",
            0,
            &["1001".into()],
            "fp1",
            "{}",
        )
        .unwrap();
        assert!(matches!(ok, CasOutcome::Ok { .. }));
        assert!(list_pending(&books).unwrap().is_empty());
    }

    #[test]
    fn idempotent_retry_after_hive_write() {
        let mut hive = MemoryHive::with_tab(KIND_VOUCHER, &["serial_no"]);
        hive.cas_row(KIND_VOUCHER, "20", "", 0, &["20".into()], "done")
            .unwrap();
        let retry = hive
            .cas_row(KIND_VOUCHER, "20", "old", 0, &["20".into()], "done")
            .unwrap();
        match retry {
            CasOutcome::Ok { fp, rev, .. } => {
                assert_eq!(fp, "done");
                assert_eq!(rev, 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dirty_keys_include_pending_and_vouchers() {
        let books = open_memory().unwrap();
        enqueue_submit(&books, "PUR-0001", KIND_PURCHASE, "{}", "", 0).unwrap();
        let keys = list_dirty_keys(&books).unwrap();
        assert!(keys.iter().any(|k| k.kind == KIND_PURCHASE && k.key == "PUR-0001"));
    }

    #[test]
    fn conflict_copy_uses_key_not_only_voucher() {
        assert_eq!(
            hive_conflict_message("PAY-0001"),
            "PAY-0001 was just updated by another user. Reload and submit again."
        );
        assert_eq!(
            hive_conflict_message("Voucher 20"),
            "Voucher 20 was just updated by another user. Reload and submit again."
        );
    }

    #[test]
    fn windows_hive_kinds_cover_every_office_module() {
        assert_eq!(HIVE_KINDS.len(), 9);
        assert!(HIVE_KINDS.contains(&KIND_SALES_PO));
        assert_eq!(tab_name(KIND_SALES_PO), "Sales_PO");
        assert_eq!(sales_po_hive_key("PO-1", "CUST_01"), "PO-1@CUST_01");
    }
}
