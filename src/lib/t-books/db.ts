import type { Database, SqlValue } from "sql.js";
import { IDB_NAME, IDB_SQLITE_KEY, IDB_STORE } from "./constants";
import { OFFICE_COLUMN_MIGRATIONS, SCHEMA_SQL, VOUCHER_COLUMN_MIGRATIONS, appVersionSql, logicVersionSql, schemaVersionSql } from "./schema";

let db: Database | null = null;
let persistTimer: ReturnType<typeof setTimeout> | null = null;

function requireWindow(): void {
  if (typeof window === "undefined") {
    throw new Error("The local books open on this PC only.");
  }
}

function openIdb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, 2);
    req.onupgradeneeded = () => {
      const store = req.result;
      if (!store.objectStoreNames.contains(IDB_STORE)) {
        store.createObjectStore(IDB_STORE);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () =>
      reject(req.error ?? new Error("Could not open the local books store on this PC."));
  });
}

async function loadBytes(): Promise<Uint8Array | null> {
  const idb = await openIdb();
  try {
    return await new Promise((resolve, reject) => {
      const tx = idb.transaction(IDB_STORE, "readonly");
      const req = tx.objectStore(IDB_STORE).get(IDB_SQLITE_KEY);
      req.onsuccess = () => {
        const value = req.result;
        if (!value) {
          resolve(null);
          return;
        }
        if (value instanceof Uint8Array) {
          resolve(value);
          return;
        }
        if (value instanceof ArrayBuffer) {
          resolve(new Uint8Array(value));
          return;
        }
        reject(new Error("Local books file on this PC is not readable."));
      };
      req.onerror = () =>
        reject(req.error ?? new Error("Could not read the local books on this PC."));
    });
  } finally {
    idb.close();
  }
}

async function saveBytes(data: Uint8Array): Promise<void> {
  const idb = await openIdb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = idb.transaction(IDB_STORE, "readwrite");
      tx.oncomplete = () => resolve();
      tx.onerror = () =>
        reject(tx.error ?? new Error("Could not save the local books on this PC."));
      tx.objectStore(IDB_STORE).put(data, IDB_SQLITE_KEY);
    });
  } finally {
    idb.close();
  }
}

async function persistNow(): Promise<void> {
  if (!db) return;
  await saveBytes(db.export());
}

function schedulePersist(): void {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    void persistNow().catch((err) => {
      console.error("[t-books] save failed:", err instanceof Error ? err.message : "unknown");
    });
  }, 40);
}

function wasmUrlFor(): string {
  return "/sql-wasm.wasm";
}

async function loadWasmBinary(): Promise<ArrayBuffer> {
  const response = await fetch(wasmUrlFor());
  if (!response.ok) {
    throw new Error("Could not load the local books engine on this PC.");
  }
  return response.arrayBuffer();
}

export async function openLocalBooks(): Promise<void> {
  requireWindow();
  if (db) return;

  const initSqlJs = (await import("sql.js")).default;
  const wasmBinary = await loadWasmBinary();
  const SQL = await initSqlJs({
    wasmBinary,
    locateFile: wasmUrlFor,
  });

  const existing = await loadBytes();
  db = existing ? new SQL.Database(existing) : new SQL.Database();
  db.exec(SCHEMA_SQL);
  migrateVoucherColumns();
  migrateOfficeColumns();
  const version = schemaVersionSql();
  db.run(version.sql, version.params);
  const logic = logicVersionSql();
  db.run(logic.sql, logic.params);
  const appVer = appVersionSql();
  db.run(appVer.sql, appVer.params);
  db.run(`INSERT OR IGNORE INTO app_meta (key, value) VALUES ('auto_backup', 'yes')`);
  await persistNow();
}

function migrateVoucherColumns(): void {
  const info = all<{ name: string }>("PRAGMA table_info(vouchers)");
  const names = new Set(info.map((row) => String(row.name)));
  for (const col of VOUCHER_COLUMN_MIGRATIONS) {
    if (names.has(col.name)) continue;
    getDb().run(`ALTER TABLE vouchers ADD COLUMN ${col.name} ${col.decl}`);
  }
  getDb().run(
    "UPDATE vouchers SET is_dirty = 1 WHERE COALESCE(dirty, 0) = 1 AND COALESCE(is_dirty, 0) = 0",
  );
  getDb().run(
    "UPDATE vouchers SET source_hash = fingerprint WHERE (source_hash IS NULL OR source_hash = '') AND fingerprint IS NOT NULL AND fingerprint != ''",
  );
}

function migrateOfficeColumns(): void {
  for (const col of OFFICE_COLUMN_MIGRATIONS) {
    const info = all<{ name: string }>(`PRAGMA table_info(${col.table})`);
    const names = new Set(info.map((row) => String(row.name)));
    if (names.has(col.name)) continue;
    getDb().run(`ALTER TABLE ${col.table} ADD COLUMN ${col.name} ${col.decl}`);
  }
}

export function getDb(): Database {
  if (!db) throw new Error("Local books are not open yet.");
  return db;
}

export function all<T extends Record<string, SqlValue>>(
  sql: string,
  params: SqlValue[] = [],
): T[] {
  const stmt = getDb().prepare(sql);
  try {
    stmt.bind(params);
    const rows: T[] = [];
    while (stmt.step()) rows.push(stmt.getAsObject() as T);
    return rows;
  } finally {
    stmt.free();
  }
}

export function run(sql: string, params: SqlValue[] = []): void {
  getDb().run(sql, params);
  schedulePersist();
}

/** Direct exec — no persist. Use inside withTransaction. */
export function exec(sql: string, params: SqlValue[] = []): void {
  getDb().run(sql, params);
}

export function withTransaction(work: () => void): void {
  const database = getDb();
  database.run("BEGIN IMMEDIATE");
  try {
    work();
    database.run("COMMIT");
  } catch (err) {
    try {
      database.run("ROLLBACK");
    } catch {
      /* keep original */
    }
    throw err;
  }
  schedulePersist();
}

export function lastInsertId(): number {
  const rows = all<{ id: number }>("SELECT last_insert_rowid() AS id");
  return Number(rows[0]?.id ?? 0);
}

export function getMeta(key: string): string | null {
  const rows = all<{ value: string }>(`SELECT value FROM app_meta WHERE key = ? LIMIT 1`, [key]);
  const value = rows[0]?.value;
  return value ? String(value) : null;
}

export async function flushLocalBooks(): Promise<void> {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }
  await persistNow();
}

export function exportDbBytes(): Uint8Array {
  return getDb().export();
}

export async function replaceFromBytes(bytes: Uint8Array): Promise<void> {
  const initSqlJs = (await import("sql.js")).default;
  const wasmBinary = await loadWasmBinary();
  const SQL = await initSqlJs({ wasmBinary, locateFile: wasmUrlFor });
  db = new SQL.Database(bytes);
  db.exec(SCHEMA_SQL);
  migrateVoucherColumns();
  migrateOfficeColumns();
  const version = schemaVersionSql();
  db.run(version.sql, version.params);
  const logic = logicVersionSql();
  db.run(logic.sql, logic.params);
  const appVer = appVersionSql();
  db.run(appVer.sql, appVer.params);
  db.run(`INSERT OR IGNORE INTO app_meta (key, value) VALUES ('auto_backup', 'yes')`);
  await persistNow();
}
