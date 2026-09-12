import { APP_VERSION, LOOPBOOK_LOGIC_VERSION, SCHEMA_VERSION } from "./constants";

/** Local-only modules. Refresh must never DELETE or rewrite these tables. */
export const OFFICE_SCHEMA_SQL = `
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
`;

export const SCHEMA_SQL = `
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

${OFFICE_SCHEMA_SQL}
`;

export function schemaVersionSql(): { sql: string; params: string[] } {
  return {
    sql: `INSERT OR REPLACE INTO app_meta (key, value) VALUES ('schema_version', ?)`,
    params: [String(SCHEMA_VERSION)],
  };
}

export function logicVersionSql(): { sql: string; params: string[] } {
  return {
    sql: `INSERT OR IGNORE INTO app_meta (key, value) VALUES ('loopbook_logic_version', ?)`,
    params: [LOOPBOOK_LOGIC_VERSION],
  };
}

export function appVersionSql(): { sql: string; params: string[] } {
  return {
    sql: `INSERT OR REPLACE INTO app_meta (key, value) VALUES ('version', ?)`,
    params: [APP_VERSION],
  };
}

export const VOUCHER_COLUMN_MIGRATIONS: Array<{ name: string; decl: string }> = [
  { name: "is_dirty", decl: "INTEGER NOT NULL DEFAULT 0" },
  { name: "total_value", decl: "REAL" },
  { name: "total_paid", decl: "REAL" },
  { name: "remaining", decl: "REAL" },
  { name: "status", decl: "TEXT" },
  { name: "tax_flag", decl: "TEXT" },
  { name: "source_hash", decl: "TEXT" },
  { name: "hive_rev", decl: "INTEGER NOT NULL DEFAULT 0" },
];

export const OFFICE_COLUMN_MIGRATIONS: Array<{ table: string; name: string; decl: string }> = [
  { table: "sales_po_items", name: "item_name", decl: "TEXT" },
  { table: "purchase_po_items", name: "item_name", decl: "TEXT" },
  { table: "purchase_po", name: "goods_received", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "purchase_po", name: "tax_invoice_no", decl: "TEXT" },
  { table: "purchase_po", name: "tax_invoice_date", decl: "TEXT" },
  { table: "purchase_po", name: "is_dirty", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "purchase_po", name: "hive_rev", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "purchase_po", name: "source_hash", decl: "TEXT" },
  { table: "hr_people", name: "active", decl: "TEXT NOT NULL DEFAULT 'Yes'" },
  { table: "hr_payroll", name: "salary_number", decl: "TEXT" },
  { table: "hr_payroll", name: "pay_kind", decl: "TEXT NOT NULL DEFAULT 'salary'" },
  { table: "hr_payroll", name: "salary_rupees", decl: "REAL NOT NULL DEFAULT 0" },
  { table: "hr_payroll", name: "recovery_rupees", decl: "REAL NOT NULL DEFAULT 0" },
  { table: "hr_payroll", name: "net_rupees", decl: "REAL NOT NULL DEFAULT 0" },
  { table: "hr_payroll", name: "is_dirty", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "hr_payroll", name: "hive_rev", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "inventory", name: "is_dirty", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "inventory", name: "hive_rev", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "inventory", name: "source_hash", decl: "TEXT" },
  { table: "logistics", name: "is_dirty", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "logistics", name: "hive_rev", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "logistics", name: "source_hash", decl: "TEXT" },
  { table: "documents", name: "is_dirty", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "documents", name: "hive_rev", decl: "INTEGER NOT NULL DEFAULT 0" },
  { table: "documents", name: "source_hash", decl: "TEXT" },
];
