/** Loopbook Control modes — SQLite only. Views never call Google. */

import { all, exec } from "./db";
import { HIVE_KINDS, tabName } from "./hive";
import { EXPLORER_TABLES, SECRET_COLUMN_RE, type ExplorerTable } from "./modules";
import { invokeCommand, isTauriRuntime } from "./platform";
import type { CasOutcome } from "./types";

export type DuplicateSerial = { voucherNo: string; count: number };
export type DuplicateVariant = { id: string; name: string };
export type DuplicateGroup = { key: string; variants: DuplicateVariant[] };
export type DuplicatesReport = {
  serials: DuplicateSerial[];
  vendors: DuplicateGroup[];
  projects: DuplicateGroup[];
};

export type RuleRow = {
  id: string;
  name: string;
  whenType: string;
  thenAction: string;
  enabled: boolean;
  sortOrder: number;
};

export type HiveTabStatus = {
  kind: string;
  tab: string;
  present: boolean;
  rowCount: number;
  writable?: boolean;
  error: string | null;
};

export type HiveStatus = {
  online: boolean;
  credentialsFound: boolean;
  tabs: HiveTabStatus[];
};

type SqlRow = Record<string, string | number | null | Uint8Array>;

function text(row: SqlRow, key: string): string {
  const value = row[key];
  return value == null || value instanceof Uint8Array ? "" : String(value);
}

function num(row: SqlRow, key: string): number {
  const value = row[key];
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const n = Number(value);
    return Number.isFinite(n) ? n : 0;
  }
  return 0;
}

function lookalikeKey(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function groupLookalikes(rows: Array<{ id: string; name: string }>): DuplicateGroup[] {
  const map = new Map<string, DuplicateVariant[]>();
  for (const row of rows) {
    const key = lookalikeKey(row.name);
    if (!key) continue;
    const list = map.get(key) ?? [];
    list.push(row);
    map.set(key, list);
  }
  return [...map.entries()]
    .filter(([, variants]) => new Set(variants.map((v) => v.name.toLowerCase())).size > 1)
    .map(([key, variants]) => ({ key, variants }));
}

function listDuplicatesLocal(): DuplicatesReport {
  const serials = all<SqlRow>(
    `SELECT CAST(voucher_number AS TEXT) AS voucher_no, COUNT(*) AS c
     FROM vouchers GROUP BY voucher_number HAVING COUNT(*) > 1
     ORDER BY COUNT(*) DESC, voucher_number`,
  ).map((row) => ({ voucherNo: text(row, "voucher_no"), count: num(row, "c") }));
  const vendors = all<SqlRow>("SELECT vendor FROM vendors").map((row) => ({
    id: text(row, "vendor"),
    name: text(row, "vendor"),
  }));
  const projects = all<SqlRow>("SELECT project FROM projects").map((row) => ({
    id: text(row, "project"),
    name: text(row, "project"),
  }));
  return {
    serials,
    vendors: groupLookalikes(vendors),
    projects: groupLookalikes(projects),
  };
}

function mergeMasterLocal(kind: string, keepId: string, absorbIds: string[]): number {
  const keep = keepId.trim();
  const absorb = absorbIds.map((s) => s.trim()).filter((s) => s && s.toLowerCase() !== keep.toLowerCase());
  if (!keep) throw new Error("Keep name is required.");
  if (!absorb.length) return 0;
  if (kind === "vendor") {
    for (const id of absorb) {
      exec("UPDATE vouchers SET vendor = ? WHERE vendor = ? COLLATE NOCASE", [keep, id]);
      exec("UPDATE purchase_po SET vendor = ? WHERE vendor = ? COLLATE NOCASE", [keep, id]);
      exec("UPDATE purchase_payments SET vendor = ? WHERE vendor = ? COLLATE NOCASE", [keep, id]);
      exec("DELETE FROM vendors WHERE vendor = ? COLLATE NOCASE AND vendor != ? COLLATE NOCASE", [id, keep]);
    }
    return absorb.length;
  }
  if (kind === "project") {
    for (const id of absorb) {
      exec("UPDATE vouchers SET project = ? WHERE project = ? COLLATE NOCASE", [keep, id]);
      exec(
        `UPDATE sales_po SET project = ? WHERE project = ? COLLATE NOCASE
         AND NOT EXISTS (
           SELECT 1 FROM sales_po k WHERE k.project = ? COLLATE NOCASE
             AND lower(k.po_number) = lower(sales_po.po_number)
         )`,
        [keep, id, keep],
      );
      exec("DELETE FROM sales_po WHERE project = ? COLLATE NOCASE", [id]);
      exec(
        `UPDATE purchase_po SET project = ? WHERE project = ? COLLATE NOCASE
         AND NOT EXISTS (
           SELECT 1 FROM purchase_po k WHERE k.project = ? COLLATE NOCASE
             AND lower(k.po_number) = lower(purchase_po.po_number)
         )`,
        [keep, id, keep],
      );
      exec("DELETE FROM purchase_po WHERE project = ? COLLATE NOCASE", [id]);
      exec("UPDATE inventory SET project = ? WHERE project = ? COLLATE NOCASE", [keep, id]);
      exec("UPDATE logistics SET project = ? WHERE project = ? COLLATE NOCASE", [keep, id]);
      exec("DELETE FROM projects WHERE project = ? COLLATE NOCASE AND project != ? COLLATE NOCASE", [id, keep]);
    }
    return absorb.length;
  }
  throw new Error("Merge is only for vendor or project names.");
}

function exploreTableLocal(table: string): Array<Record<string, string>> {
  if (!(EXPLORER_TABLES as readonly string[]).includes(table)) {
    throw new Error("Unknown table");
  }
  const info = all<SqlRow>(`PRAGMA table_info(${table})`);
  const cols = info.map((row) => text(row, "name")).filter((name) => !SECRET_COLUMN_RE.test(name));
  if (!cols.length) return [];
  const quoted = cols.map((c) => `"${c}"`).join(", ");
  return all<SqlRow>(`SELECT ${quoted} FROM ${table} LIMIT 200`).map((row) => {
    const out: Record<string, string> = {};
    for (const col of cols) out[col] = text(row, col);
    return out;
  });
}

function listRulesLocal(): RuleRow[] {
  return all<SqlRow>("SELECT id, name, when_type, then_action, enabled, sort_order FROM rules ORDER BY sort_order, id").map(
    (row) => ({
      id: text(row, "id"),
      name: text(row, "name"),
      whenType: text(row, "when_type"),
      thenAction: text(row, "then_action"),
      enabled: num(row, "enabled") !== 0,
      sortOrder: num(row, "sort_order"),
    }),
  );
}

function toggleRuleLocal(id: string, enabled: boolean): RuleRow {
  exec("UPDATE rules SET enabled = ? WHERE id = ?", [enabled ? 1 : 0, id]);
  const row = listRulesLocal().find((r) => r.id === id);
  if (!row) throw new Error("That rule is not on this PC.");
  return row;
}

async function call<T>(command: string, local: () => T, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) return invokeCommand<T>(command, args);
  return local();
}

export async function listDuplicates(): Promise<DuplicatesReport> {
  return call("list_duplicates", listDuplicatesLocal);
}

export async function mergeMaster(kind: "vendor" | "project", keepId: string, absorbIds: string[]): Promise<number> {
  return call("merge_master", () => mergeMasterLocal(kind, keepId, absorbIds), {
    kind,
    keepId,
    absorbIds,
  });
}

export async function exploreTable(table: ExplorerTable): Promise<Array<Record<string, string>>> {
  return call("explore_table", () => exploreTableLocal(table), { table });
}

export async function listRules(): Promise<RuleRow[]> {
  return call("list_rules", listRulesLocal);
}

export async function toggleRule(id: string, enabled: boolean): Promise<RuleRow> {
  return call("toggle_rule", () => toggleRuleLocal(id, enabled), { id, enabled });
}

export async function hiveStatus(): Promise<HiveStatus> {
  return call("hive_status", () => ({
    online: typeof navigator !== "undefined" ? navigator.onLine : false,
    credentialsFound: false,
    tabs: HIVE_KINDS.map((kind) => ({
      kind,
      tab: tabName(kind),
      present: false,
      rowCount: 0,
      writable: true,
      error: "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
    })),
  }));
}

export async function bootstrapHiveTab(kind: string): Promise<string> {
  return call("bootstrap_hive_tab", () => {
    throw new Error("Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.");
  }, { kind });
}

export async function retryPendingSubmit(kind: string, key: string): Promise<CasOutcome> {
  return call("retry_pending_submit", () => {
    throw new Error("Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.");
  }, { kind, key });
}

export async function listPendingSubmit() {
  return call("list_pending_submit", () => {
    return all<SqlRow>(
      "SELECT key, kind, payload, base_fp, base_rev, attempts, COALESCE(last_error,'') AS last_error FROM pending_submit ORDER BY kind, key",
    ).map((row) => ({
      key: text(row, "key"),
      kind: text(row, "kind"),
      payload: text(row, "payload"),
      baseFp: text(row, "base_fp"),
      baseRev: num(row, "base_rev"),
      attempts: num(row, "attempts"),
      lastError: text(row, "last_error"),
    }));
  });
}

export { EXPLORER_TABLES };
export type { ExplorerTable };
