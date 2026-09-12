/** Hive contract (preview twin). Views never call Google. */

export const KIND_VOUCHER = "voucher";
export const KIND_PURCHASE = "purchase";
export const KIND_PAYMENT = "payment";
export const KIND_SALARY = "salary";
export const KIND_SALES_PO = "sales_po";
export const KIND_ACCESS = "access";

export type DirtyKey = { kind: string; key: string };

export type HiveRow = {
  key: string;
  kind: string;
  fp: string;
  rev: number;
  cells: string[];
};

export type CasOutcome =
  | { kind: "ok"; key: string; fp: string; rev: number }
  | { kind: "conflict"; key: string; message: string };

export function hiveConflictMessage(key: string): string {
  return `${key} was just updated by another user. Reload and submit again.`;
}

export function salesPoHiveKey(poNumber: string, project: string): string {
  return `${poNumber.trim()}@${project.trim()}`;
}

type Tab = { headers: string[]; rows: HiveRow[] };

export class MemoryHive {
  tabs = new Map<string, Tab>();
  failWrites = false;

  getRow(kind: string, key: string): HiveRow | null {
    const tab = this.tabs.get(kind);
    if (!tab) return null;
    return tab.rows.find((r) => r.key === key) ?? null;
  }

  listKeys(kind: string): string[] {
    return (this.tabs.get(kind)?.rows ?? []).map((r) => r.key);
  }

  ensureTab(kind: string, headers: string[]): "exists" | "bootstrapped" {
    if (this.tabs.has(kind)) return "exists";
    this.tabs.set(kind, { headers: [...headers], rows: [] });
    return "bootstrapped";
  }

  casRow(
    kind: string,
    key: string,
    baseFp: string,
    baseRev: number,
    cells: string[],
    newFp: string,
  ): CasOutcome {
    if (this.failWrites) {
      throw new Error("Could not reach Google Sheets from this PC.");
    }
    const tab = this.tabs.get(kind);
    if (!tab) {
      throw new Error(`Hive tab ${kind} is missing. Owner can bootstrap headers only.`);
    }
    const idx = tab.rows.findIndex((r) => r.key === key);
    if (idx < 0) {
      if (baseRev !== 0 && baseFp) {
        return { kind: "conflict", key, message: hiveConflictMessage(key) };
      }
      tab.rows.push({ key, kind, fp: newFp, rev: 1, cells: [...cells] });
      return { kind: "ok", key, fp: newFp, rev: 1 };
    }
    const row = tab.rows[idx]!;
    if (row.fp === newFp) {
      return { kind: "ok", key, fp: row.fp, rev: row.rev };
    }
    if (row.fp !== baseFp || row.rev !== baseRev) {
      return { kind: "conflict", key, message: hiveConflictMessage(key) };
    }
    tab.rows[idx] = { key, kind, fp: newFp, rev: baseRev + 1, cells: [...cells] };
    return { kind: "ok", key, fp: newFp, rev: baseRev + 1 };
  }
}
