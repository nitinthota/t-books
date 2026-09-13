use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{DATA_FOLDER_NAME, DB_FILE_NAME, LOOPBOOK_LOGIC_VERSION, SCHEMA_VERSION, Result};

pub struct LocalBooks {
    conn: Connection,
    path: PathBuf,
}

impl LocalBooks {
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Windows: %LOCALAPPDATA%\T-Books
pub fn data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    let dir = base.join(DATA_FOLDER_NAME);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn db_path() -> PathBuf {
    data_dir().join(DB_FILE_NAME)
}

pub fn open_db() -> Result<LocalBooks> {
    let path = db_path();
    let conn = Connection::open(&path)?;
    migrate(&conn)?;
    Ok(LocalBooks { conn, path })
}

pub fn open_at(path: &Path) -> Result<LocalBooks> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(LocalBooks {
        conn,
        path: path.to_path_buf(),
    })
}

pub fn open_memory() -> Result<LocalBooks> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(LocalBooks {
        conn,
        path: PathBuf::from(":memory:"),
    })
}

fn column_names(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut names = Vec::new();
    for row in rows {
        names.push(row?);
    }
    Ok(names)
}

fn seed_default_rules(conn: &Connection) -> Result<()> {
    const RULES: &[(&str, &str, &str, &str, i64)] = &[
        (
            "ledger-purchase",
            "Post vendor bills to ledger",
            "purchase",
            "ledger",
            1,
        ),
        (
            "ledger-payment",
            "Post vendor payments to bank",
            "payment",
            "bank",
            2,
        ),
        (
            "project-cost",
            "Add bill value to project cost",
            "purchase",
            "project_cost",
            3,
        ),
        (
            "stock-inventory",
            "Move stock when inventory posts",
            "inventory",
            "stock",
            4,
        ),
        (
            "ledger-salary",
            "Post salary to payroll ledger",
            "salary",
            "ledger",
            5,
        ),
        (
            "ledger-sale",
            "Post sales PO received to ledger",
            "sale",
            "ledger",
            6,
        ),
    ];
    for (id, name, when_type, then_action, sort) in RULES {
        conn.execute(
            "INSERT OR IGNORE INTO rules (id, name, when_type, then_action, enabled, sort_order)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            rusqlite::params![id, name, when_type, then_action, sort],
        )?;
    }
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, name: &str, decl: &str) -> Result<()> {
    let cols = column_names(conn, table)?;
    if cols.iter().any(|c| c == name) {
        return Ok(());
    }
    conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {name} {decl}"), [])?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE IF NOT EXISTS users_local (
          email TEXT PRIMARY KEY COLLATE NOCASE,
          password_hash TEXT NOT NULL,
          created_at TEXT
        );

        CREATE TABLE IF NOT EXISTS app_meta (
          key TEXT PRIMARY KEY,
          value TEXT
        );

        CREATE TABLE IF NOT EXISTS access_cache (
          email TEXT PRIMARY KEY COLLATE NOCASE,
          name TEXT,
          role TEXT,
          active TEXT,
          last_synced TEXT
        );

        CREATE TABLE IF NOT EXISTS vendors (
          vendor TEXT PRIMARY KEY COLLATE NOCASE,
          bank TEXT,
          account_number TEXT,
          ifsc TEXT,
          gst TEXT,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS projects (
          project TEXT PRIMARY KEY COLLATE NOCASE,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS vouchers (
          voucher_number INTEGER PRIMARY KEY,
          voucher_date TEXT,
          tax_invoice TEXT,
          vendor TEXT,
          bank TEXT,
          account_number TEXT,
          ifsc TEXT,
          gst TEXT,
          project TEXT,
          comments TEXT,
          description TEXT,
          dirty INTEGER NOT NULL DEFAULT 0,
          is_dirty INTEGER NOT NULL DEFAULT 0,
          source TEXT,
          fingerprint TEXT,
          source_hash TEXT,
          total_value REAL,
          total_paid REAL,
          remaining REAL,
          status TEXT,
          tax_flag TEXT,
          created_at TEXT,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS voucher_payments (
          voucher_number INTEGER NOT NULL,
          slot INTEGER NOT NULL CHECK (slot >= 1 AND slot <= 5),
          pi_no TEXT,
          pi_date TEXT,
          pi_value_rupees REAL,
          payment_percent REAL,
          description TEXT,
          tds_rupees REAL,
          paid_rupees REAL,
          payment_details TEXT,
          payment_date TEXT,
          remaining_rupees REAL,
          remarks TEXT,
          PRIMARY KEY (voucher_number, slot),
          FOREIGN KEY (voucher_number) REFERENCES vouchers(voucher_number)
        );

        CREATE INDEX IF NOT EXISTS idx_vouchers_is_dirty ON vouchers(is_dirty);

        CREATE TABLE IF NOT EXISTS sales_po (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          project TEXT NOT NULL DEFAULT '',
          po_number TEXT NOT NULL DEFAULT '',
          client TEXT,
          gst TEXT,
          total_value REAL,
          created_at TEXT,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sales_po_items (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          po_id INTEGER NOT NULL,
          description TEXT,
          qty REAL,
          rate REAL,
          gst_pct REAL NOT NULL DEFAULT 0,
          amount REAL,
          FOREIGN KEY (po_id) REFERENCES sales_po(id) ON DELETE CASCADE
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_sales_po_project_number
          ON sales_po(project COLLATE NOCASE, po_number COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_sales_po_project ON sales_po(project);
        CREATE INDEX IF NOT EXISTS idx_sales_po_items_po ON sales_po_items(po_id);

        CREATE TABLE IF NOT EXISTS purchase_po (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          project TEXT NOT NULL DEFAULT '',
          vendor TEXT,
          po_number TEXT NOT NULL DEFAULT '',
          type TEXT NOT NULL DEFAULT 'contract',
          total_value REAL,
          created_at TEXT,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS purchase_po_items (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          po_id INTEGER NOT NULL,
          description TEXT,
          qty REAL,
          rate REAL,
          gst_pct REAL NOT NULL DEFAULT 0,
          amount REAL,
          FOREIGN KEY (po_id) REFERENCES purchase_po(id) ON DELETE CASCADE
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_purchase_po_project_number
          ON purchase_po(project COLLATE NOCASE, po_number COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_purchase_po_project ON purchase_po(project);
        CREATE INDEX IF NOT EXISTS idx_purchase_po_vendor ON purchase_po(vendor);
        CREATE INDEX IF NOT EXISTS idx_purchase_po_items_po ON purchase_po_items(po_id);

        CREATE TABLE IF NOT EXISTS hr_people (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          role TEXT,
          salary REAL
        );

        CREATE TABLE IF NOT EXISTS hr_payroll (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          person_id INTEGER NOT NULL,
          month TEXT NOT NULL,
          pf_employee REAL,
          pf_company REAL,
          tds REAL,
          total_paid REAL,
          FOREIGN KEY (person_id) REFERENCES hr_people(id) ON DELETE CASCADE
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_hr_payroll_person_month
          ON hr_payroll(person_id, month);
        CREATE INDEX IF NOT EXISTS idx_hr_people_name ON hr_people(name);

        CREATE TABLE IF NOT EXISTS inventory (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          item_name TEXT NOT NULL,
          type TEXT,
          size TEXT,
          quantity REAL,
          cost REAL,
          project TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_inventory_item ON inventory(item_name);
        CREATE INDEX IF NOT EXISTS idx_inventory_type ON inventory(type);
        CREATE INDEX IF NOT EXISTS idx_inventory_project ON inventory(project);

        CREATE TABLE IF NOT EXISTS logistics (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          project TEXT,
          vehicle_number TEXT,
          invoice_number TEXT,
          start_date TEXT,
          reach_date TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_logistics_project ON logistics(project);
        CREATE INDEX IF NOT EXISTS idx_logistics_vehicle ON logistics(vehicle_number);

        CREATE TABLE IF NOT EXISTS documents (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT,
          path TEXT,
          linked_type TEXT,
          linked_id TEXT,
          created_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_documents_linked ON documents(linked_type, linked_id);
        CREATE INDEX IF NOT EXISTS idx_documents_name ON documents(name);

        CREATE TABLE IF NOT EXISTS pending_submit (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          key TEXT NOT NULL,
          kind TEXT NOT NULL,
          payload TEXT NOT NULL DEFAULT '{}',
          base_fp TEXT NOT NULL DEFAULT '',
          base_rev INTEGER NOT NULL DEFAULT 0,
          attempts INTEGER NOT NULL DEFAULT 0,
          last_error TEXT,
          created_at TEXT,
          updated_at TEXT,
          UNIQUE(key, kind)
        );
        CREATE INDEX IF NOT EXISTS idx_pending_submit_kind ON pending_submit(kind);

        CREATE TABLE IF NOT EXISTS purchase_payments (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          pay_number TEXT NOT NULL,
          po_number TEXT NOT NULL DEFAULT '',
          vendor TEXT,
          project TEXT,
          amount_rupees REAL NOT NULL DEFAULT 0,
          alloc_method TEXT NOT NULL DEFAULT 'advance',
          pay_class TEXT NOT NULL DEFAULT 'advance',
          missing_tax_invoice INTEGER NOT NULL DEFAULT 0,
          pay_date TEXT,
          remarks TEXT,
          is_dirty INTEGER NOT NULL DEFAULT 1,
          hive_rev INTEGER NOT NULL DEFAULT 0,
          source_hash TEXT,
          created_at TEXT,
          updated_at TEXT
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_purchase_payments_number
          ON purchase_payments(pay_number COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_purchase_payments_po ON purchase_payments(po_number);

        CREATE TABLE IF NOT EXISTS rules (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          when_type TEXT NOT NULL,
          then_action TEXT NOT NULL,
          enabled INTEGER NOT NULL DEFAULT 1,
          sort_order INTEGER NOT NULL DEFAULT 0
        );
        "#,
    )?;
    seed_default_rules(conn)?;
    ensure_column(conn, "vouchers", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "vouchers", "total_value", "REAL")?;
    ensure_column(conn, "vouchers", "total_paid", "REAL")?;
    ensure_column(conn, "vouchers", "remaining", "REAL")?;
    ensure_column(conn, "vouchers", "status", "TEXT")?;
    ensure_column(conn, "vouchers", "tax_flag", "TEXT")?;
    ensure_column(conn, "vouchers", "source_hash", "TEXT")?;
    ensure_column(conn, "vouchers", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    conn.execute(
        "UPDATE vouchers SET source_hash = fingerprint WHERE (source_hash IS NULL OR source_hash = '') AND fingerprint IS NOT NULL AND fingerprint != ''",
        [],
    )?;
    ensure_column(conn, "sales_po_items", "gst_pct", "REAL NOT NULL DEFAULT 0")?;
    ensure_column(conn, "purchase_po_items", "gst_pct", "REAL NOT NULL DEFAULT 0")?;
    ensure_column(conn, "sales_po_items", "item_name", "TEXT")?;
    ensure_column(conn, "purchase_po_items", "item_name", "TEXT")?;
    ensure_column(conn, "purchase_po", "goods_received", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "purchase_po", "tax_invoice_no", "TEXT")?;
    ensure_column(conn, "purchase_po", "tax_invoice_date", "TEXT")?;
    ensure_column(conn, "purchase_po", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "purchase_po", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "purchase_po", "source_hash", "TEXT")?;
    ensure_column(conn, "sales_po", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "sales_po", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "sales_po", "source_hash", "TEXT")?;
    ensure_column(conn, "hr_people", "active", "TEXT NOT NULL DEFAULT 'Yes'")?;
    ensure_column(conn, "hr_payroll", "salary_number", "TEXT")?;
    ensure_column(conn, "hr_payroll", "pay_kind", "TEXT NOT NULL DEFAULT 'salary'")?;
    ensure_column(conn, "hr_payroll", "salary_rupees", "REAL NOT NULL DEFAULT 0")?;
    ensure_column(conn, "hr_payroll", "recovery_rupees", "REAL NOT NULL DEFAULT 0")?;
    ensure_column(conn, "hr_payroll", "net_rupees", "REAL NOT NULL DEFAULT 0")?;
    ensure_column(conn, "hr_payroll", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "hr_payroll", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "hr_payroll", "source_hash", "TEXT")?;
    ensure_column(conn, "inventory", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "inventory", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "inventory", "source_hash", "TEXT")?;
    ensure_column(conn, "logistics", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "logistics", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "logistics", "source_hash", "TEXT")?;
    ensure_column(conn, "documents", "is_dirty", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "documents", "hive_rev", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "documents", "source_hash", "TEXT")?;
    conn.execute(
        "UPDATE vouchers SET is_dirty = 1 WHERE COALESCE(dirty, 0) = 1 AND COALESCE(is_dirty, 0) = 0",
        [],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('schema_version', ?1)",
        rusqlite::params![SCHEMA_VERSION],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO app_meta (key, value) VALUES ('loopbook_logic_version', ?1)",
        rusqlite::params![LOOPBOOK_LOGIC_VERSION],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('version', ?1)",
        rusqlite::params![crate::APP_VERSION],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO app_meta (key, value) VALUES ('auto_backup', 'yes')",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_required_tables() {
        let books = open_memory().unwrap();
        let count: i64 = books
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'users_local', 'app_meta', 'access_cache',
                    'vendors', 'projects', 'vouchers', 'voucher_payments',
                    'sales_po', 'sales_po_items', 'purchase_po', 'purchase_po_items',
                    'hr_people', 'hr_payroll', 'inventory', 'logistics', 'documents',
                    'pending_submit', 'purchase_payments', 'rules'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 19);
        let users: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM users_local", [], |row| row.get(0))
            .unwrap();
        assert_eq!(users, 0);
        let vouchers: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |row| row.get(0))
            .unwrap();
        assert_eq!(vouchers, 0);
        let sales: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM sales_po", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sales, 0);
        let version: String = books
            .conn()
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, "9");
        let logic: String = books
            .conn()
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'loopbook_logic_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(logic, "v1");
        let has_hash: i64 = books
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('vouchers') WHERE name = 'source_hash'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_hash, 1);
    }

    #[test]
    fn migrate_does_not_wipe_existing_vouchers() {
        let books = open_memory().unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO vouchers (voucher_number, tax_invoice, vendor) VALUES (20, 'INV-20', 'VEND_02')",
                [],
            )
            .unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO sales_po (project, po_number, client, total_value) VALUES ('CUST_01', 'PO-1', 'CUST_01', 100)",
                [],
            )
            .unwrap();
        migrate(books.conn()).unwrap();
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |row| row.get(0))
            .unwrap();
        assert_eq!(n, 1);
        let po: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM sales_po", [], |row| row.get(0))
            .unwrap();
        assert_eq!(po, 1);
        let logic: String = books
            .conn()
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'loopbook_logic_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(logic, "v1");
    }
}
