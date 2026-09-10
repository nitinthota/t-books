import {
  ACCESS_TAB,
  CREDENTIALS_MISSING_ERROR,
  OFFLINE_REFRESH_ERROR,
} from "./constants";
import { all, exec, getMeta, withTransaction } from "./db";
import { isHardcodedOwner, isRole, normalizeEmail } from "./rbac";
import { isBrowserOnline } from "./online";
import type { AccessRow, AccessSnapshot, Role } from "./types";

type CacheRow = {
  name: string | null;
  email: string;
  role: string | null;
  active: string | null;
  last_synced: string | null;
};

export function normalizeActive(raw: string): "Yes" | "No" {
  const value = raw.trim().toLowerCase();
  if (value === "yes" || value === "y" || value === "true" || value === "1") return "Yes";
  return "No";
}

export function normalizeRole(raw: string): Role {
  const value = raw.trim().toLowerCase();
  if (value === "owner") return "owner";
  if (value === "admin") return "admin";
  return "operator";
}

export function isActiveYes(active: string): boolean {
  return active.trim().toLowerCase() === "yes";
}

function findCol(header: string[], names: string[]): number {
  return header.findIndex((h) => names.includes(h.trim().toLowerCase()));
}

function cell(row: string[], index: number): string {
  if (index < 0) return "";
  return row[index] ?? "";
}

export function parseAccessValues(values: string[][]): AccessRow[] {
  if (values.length === 0) {
    throw new Error(
      `${ACCESS_TAB} tab is empty. Expected a header row: Name, Email, Role, Active.`,
    );
  }
  const headerAt = values.slice(0, 10).findIndex((row) => findCol(row, ["email"]) >= 0);
  if (headerAt < 0) {
    throw new Error(`${ACCESS_TAB} tab is missing the Email column.`);
  }
  const header = values[headerAt] ?? [];
  const emailI = findCol(header, ["email"]);
  const nameI = findCol(header, ["name"]);
  const roleI = findCol(header, ["role"]);
  const activeI = findCol(header, ["active"]);
  if (emailI < 0) {
    throw new Error(`${ACCESS_TAB} tab is missing the Email column.`);
  }
  if (activeI < 0) {
    throw new Error(`${ACCESS_TAB} tab is missing the Active column.`);
  }

  const byEmail = new Map<string, AccessRow>();
  for (const row of values.slice(headerAt + 1)) {
    const email = normalizeEmail(cell(row, emailI));
    if (!email) continue;
    byEmail.set(email, {
      name: cell(row, nameI).trim(),
      email,
      role: normalizeRole(cell(row, roleI)),
      active: normalizeActive(cell(row, activeI)),
      lastSynced: "",
    });
  }
  return [...byEmail.values()];
}

export function applyAccessRows(rows: AccessRow[]): void {
  const syncedRows = all<{ now: string }>("SELECT datetime('now') AS now");
  const synced = String(syncedRows[0]?.now ?? "");
  withTransaction(() => {
    exec("DELETE FROM access_cache");
    for (const row of rows) {
      exec(
        `INSERT INTO access_cache (email, name, role, active, last_synced) VALUES (?, ?, ?, ?, ?)`,
        [row.email, row.name, row.role, row.active, synced],
      );
    }
    exec(`INSERT OR REPLACE INTO app_meta (key, value) VALUES ('access_last_synced', ?)`, [synced]);
  });
}

export function listAccess(): AccessRow[] {
  const rows = all<CacheRow>(
    `SELECT name, email, role, active, last_synced
     FROM access_cache
     ORDER BY name COLLATE NOCASE, email COLLATE NOCASE`,
  );
  return rows.map((row) => ({
    name: row.name ? String(row.name) : "",
    email: String(row.email),
    role: row.role ? String(row.role) : "",
    active: row.active ? String(row.active) : "",
    lastSynced: row.last_synced ? String(row.last_synced) : "",
  }));
}

export function getLastSynced(): string | null {
  return getMeta("access_last_synced");
}

export function credentialsFound(): boolean {
  return false;
}

export function getAccessSnapshot(): AccessSnapshot {
  return {
    rows: listAccess(),
    lastSynced: getLastSynced(),
    credentialsFound: credentialsFound(),
  };
}

export function findAccessRow(email: string): AccessRow | null {
  const normalized = normalizeEmail(email);
  const rows = all<CacheRow>(
    `SELECT name, email, role, active, last_synced
     FROM access_cache
     WHERE email = ? COLLATE NOCASE
     LIMIT 1`,
    [normalized],
  );
  const row = rows[0];
  if (!row) return null;
  return {
    name: row.name ? String(row.name) : "",
    email: String(row.email),
    role: row.role ? String(row.role) : "",
    active: row.active ? String(row.active) : "",
    lastSynced: row.last_synced ? String(row.last_synced) : "",
  };
}

export function accessDenialMessage(email: string): string | null {
  if (isHardcodedOwner(email)) return null;
  const row = findAccessRow(email);
  if (!row) return "This email is not on the Access list for this PC.";
  if (isActiveYes(row.active)) return null;
  return "This email is not active on the Access list.";
}

export function sessionRole(email: string): Role {
  if (isHardcodedOwner(email)) return "owner";
  const row = findAccessRow(email);
  if (row?.role === "admin") return "admin";
  if (row?.role === "owner") return "admin";
  if (row && isRole(row.role)) return row.role;
  return "operator";
}

export async function refreshAccess(): Promise<AccessSnapshot> {
  if (!isBrowserOnline()) {
    throw new Error(OFFLINE_REFRESH_ERROR);
  }
  throw new Error(CREDENTIALS_MISSING_ERROR);
}
