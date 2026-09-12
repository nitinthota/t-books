/** Indian FY trial. SQLite only. Must balance. */

import {
  clampAsOf,
  currentIndianFy,
  fyBounds,
  fyOptions,
  rupeesToPaise,
  signedDrCr,
  trialBalanced,
} from "./business_rules";
import { all } from "./db";
import type { TrialBalance, TrialLine } from "./types";

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

function inRange(date: string, start: string, end: string): boolean {
  const d = date.length >= 10 ? date.slice(0, 10) : date;
  if (d.length < 10) return true;
  return d >= start && d <= end;
}

function monthInRange(month: string, start: string, end: string): boolean {
  if (month.length < 7) return true;
  return inRange(`${month}-01`, start, end);
}

export function getTrialLocal(fy?: string | null, asOf?: string | null): TrialBalance {
  const fyLabel = (fy ?? "").trim() || currentIndianFy();
  const { start, end } = fyBounds(fyLabel);
  const asOfDay = clampAsOf(asOf ?? undefined, start, end, end);
  const amounts = new Map<string, { dr: number; cr: number }>();
  const add = (account: string, paise: number) => {
    const name = account.trim();
    if (!name || !paise) return;
    const { debit_paise: dr, credit_paise: cr } = signedDrCr(paise);
    const slot = amounts.get(name) ?? { dr: 0, cr: 0 };
    slot.dr += dr;
    slot.cr += cr;
    amounts.set(name, slot);
  };

  for (const row of all<SqlRow>(
    `SELECT COALESCE(project,'') AS project, COALESCE(vendor,'') AS vendor,
            COALESCE(total_value,0) AS total_value, COALESCE(total_paid,0) AS total_paid,
            COALESCE(voucher_date,'') AS voucher_date FROM vouchers`,
  )) {
    if (!inRange(text(row, "voucher_date"), start, asOfDay)) continue;
    const value = rupeesToPaise(num(row, "total_value"));
    const paid = rupeesToPaise(num(row, "total_paid"));
    add(`Project ${text(row, "project")}`, value);
    add(`Vendor ${text(row, "vendor")}`, -value);
    add(`Vendor ${text(row, "vendor")}`, paid);
    add("Bank", -paid);
  }

  for (const row of all<SqlRow>(
    `SELECT COALESCE(project,'') AS project, COALESCE(vendor,'') AS vendor,
            COALESCE(total_value,0) AS total_value, COALESCE(created_at,'') AS created_at,
            po_number FROM purchase_po`,
  )) {
    if (!inRange(text(row, "created_at"), start, asOfDay)) continue;
    const value = rupeesToPaise(num(row, "total_value"));
    add(`Project ${text(row, "project")}`, value);
    add(`Vendor ${text(row, "vendor")}`, -value);
    const paidRows = all<SqlRow>(
      "SELECT COALESCE(SUM(amount_rupees),0) AS paid FROM purchase_payments WHERE po_number = ? COLLATE NOCASE",
      [text(row, "po_number")],
    );
    const paid = rupeesToPaise(num(paidRows[0] ?? {}, "paid"));
    add(`Vendor ${text(row, "vendor")}`, paid);
    add("Bank", -paid);
  }

  for (const row of all<SqlRow>(
    `SELECT p.month, p.net_rupees, p.pf_employee, p.pf_company, p.tds, p.recovery_rupees, p.salary_rupees
     FROM hr_payroll p`,
  )) {
    if (!monthInRange(text(row, "month"), start, asOfDay)) continue;
    const salary = rupeesToPaise(num(row, "salary_rupees"));
    const pfC = rupeesToPaise(num(row, "pf_company"));
    add("Salary", salary + pfC);
    add("PF payable", -(rupeesToPaise(num(row, "pf_employee")) + pfC));
    add("TDS", -rupeesToPaise(num(row, "tds")));
    add("Staff advance", -rupeesToPaise(num(row, "recovery_rupees")));
    add("Bank", -rupeesToPaise(num(row, "net_rupees")));
  }

  const pairs = [...amounts.values()].map((v) => ({ debit_paise: v.dr, credit_paise: v.cr }));
  const balanced = pairs.length === 0 || trialBalanced(pairs);
  const lines: TrialLine[] = [...amounts.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([account, v]) => ({
      account,
      debitRupees: v.dr / 100,
      creditRupees: v.cr / 100,
    }));
  return {
    fy: fyLabel,
    asOf: asOfDay,
    start,
    end,
    lines,
    totalDebitRupees: lines.reduce((s, l) => s + l.debitRupees, 0),
    totalCreditRupees: lines.reduce((s, l) => s + l.creditRupees, 0),
    balanced,
  };
}

export function listFyLocal(): string[] {
  const dates = all<SqlRow>("SELECT COALESCE(voucher_date,'') AS d FROM vouchers")
    .map((r) => text(r, "d"))
    .filter((d) => d.length >= 10)
    .map((d) => d.slice(0, 10));
  let min: string | undefined;
  let max: string | undefined;
  for (const d of dates) {
    if (!min || d < min) min = d;
    if (!max || d > max) max = d;
  }
  return fyOptions(min ?? null, max ?? null);
}
