/** SAL-n salary slips. Port of loopbook `hr-payroll.ts`. */

export const MONTHS = [
  { n: 1, label: "January", short: "Jan" },
  { n: 2, label: "February", short: "Feb" },
  { n: 3, label: "March", short: "Mar" },
  { n: 4, label: "April", short: "Apr" },
  { n: 5, label: "May", short: "May" },
  { n: 6, label: "June", short: "Jun" },
  { n: 7, label: "July", short: "Jul" },
  { n: 8, label: "August", short: "Aug" },
  { n: 9, label: "September", short: "Sep" },
  { n: 10, label: "October", short: "Oct" },
  { n: 11, label: "November", short: "Nov" },
  { n: 12, label: "December", short: "Dec" },
] as const;

export type PayKind = "salary" | "advance";
export type PayrollStatus = "current" | "superseded";

export function monthLabel(month: number, style: "label" | "short" = "label"): string {
  const found = MONTHS.find((m) => m.n === month);
  if (!found) return "—";
  return style === "short" ? found.short : found.label;
}

export function periodLabel(month: number, year: number): string {
  const y = Math.round(year);
  if (!Number.isFinite(y) || y < 1990) return monthLabel(month);
  return `${monthLabel(month, "short")} ${y}`;
}

export function currentPeriod(today = new Date()): { month: number; year: number; pay_date: string } {
  return {
    month: today.getMonth() + 1,
    year: today.getFullYear(),
    pay_date: today.toISOString().slice(0, 10),
  };
}

export function yearOptions(from = 2020, today = new Date()): number[] {
  const max = today.getFullYear() + 1;
  const years: number[] = [];
  for (let y = max; y >= from; y -= 1) years.push(y);
  return years;
}

export function normalizePayKind(raw?: string | null): PayKind {
  return (raw ?? "").trim().toLowerCase() === "advance" ? "advance" : "salary";
}

/** Posted salary voucher numbers are kept; never steal a live number. */
export function keepPostedPayNumber(postedNo: string, requestedNo: string): string {
  const posted = (postedNo ?? "").trim();
  if (posted) return posted;
  return (requestedNo ?? "").trim();
}

export type PayrollMoney = {
  salary_paise: number;
  pf_company_paise: number;
  pf_employee_paise: number;
  pf_total_paise: number;
  tds_paise: number;
  net_paise: number;
  ctc_paise: number;
};

function paise(v: unknown): number {
  const n = typeof v === "number" ? v : Number(v);
  if (!Number.isFinite(n)) return 0;
  return Math.round(n);
}

export function payrollMoneyChanged(a: PayrollMoney, b: PayrollMoney): boolean {
  return (
    a.salary_paise !== b.salary_paise ||
    a.pf_company_paise !== b.pf_company_paise ||
    a.pf_employee_paise !== b.pf_employee_paise ||
    a.tds_paise !== b.tds_paise
  );
}

export function payrollNetOk(money: PayrollMoney): boolean {
  return money.salary_paise > 0 && money.net_paise >= 0;
}

export type SalarySlip = {
  kind: PayKind;
  gross_paise: number;
  pf_employee_paise: number;
  pf_company_paise: number;
  tds_paise: number;
  recovery_paise: number;
  net_paise: number;
};

export function outstandingAdvancePaise(
  rows: Array<{ pay_kind?: string | null; amount_paise?: number | null; recovery_paise?: number | null }>,
): number {
  let adv = 0;
  let rec = 0;
  for (const row of rows) {
    if (normalizePayKind(row.pay_kind) === "advance") adv += paise(row.amount_paise);
    rec += paise(row.recovery_paise);
  }
  return Math.max(0, adv - rec);
}

export function calcSalarySlip(input: {
  kind: PayKind | string;
  salary_paise: number;
  pf_employee_paise: number;
  pf_company_paise: number;
  tds_paise: number;
  recovery_paise: number;
  amount_paise?: number;
}): SalarySlip {
  const kind = normalizePayKind(input.kind);
  if (kind === "advance") {
    const amount = Math.max(0, paise(input.amount_paise ?? 0));
    return {
      kind,
      gross_paise: amount,
      pf_employee_paise: 0,
      pf_company_paise: 0,
      tds_paise: 0,
      recovery_paise: 0,
      net_paise: amount,
    };
  }
  const gross = Math.max(0, paise(input.salary_paise));
  const empPf = Math.max(0, paise(input.pf_employee_paise));
  const coPf = Math.max(0, paise(input.pf_company_paise));
  const tds = Math.max(0, paise(input.tds_paise));
  const maxRecovery = Math.max(0, gross - empPf - tds);
  const recovery = Math.min(maxRecovery, Math.max(0, paise(input.recovery_paise)));
  return {
    kind,
    gross_paise: gross,
    pf_employee_paise: empPf,
    pf_company_paise: coPf,
    tds_paise: tds,
    recovery_paise: recovery,
    net_paise: gross - empPf - tds - recovery,
  };
}

export type LedgerLine = {
  account_code: string;
  debit_paise: number;
  credit_paise: number;
  remarks: string;
};

export function bankAccountCode(payMode?: string | null): "1000" | "1010" {
  return (payMode ?? "").trim().toLowerCase() === "cash" ? "1000" : "1010";
}

export function salaryVoucherLines(slip: SalarySlip, payMode?: string | null): LedgerLine[] {
  const bank = bankAccountCode(payMode);
  const lines: LedgerLine[] = [];
  const push = (account_code: string, debit_paise: number, credit_paise: number, remarks: string) => {
    if (debit_paise <= 0 && credit_paise <= 0) return;
    lines.push({ account_code, debit_paise, credit_paise, remarks });
  };
  if (slip.kind === "advance") {
    push("1400", slip.net_paise, 0, "Staff advance");
    push(bank, 0, slip.net_paise, bank === "1000" ? "Cash" : "Bank");
    return lines;
  }
  const expense = slip.gross_paise + slip.pf_company_paise;
  const pfPayable = slip.pf_employee_paise + slip.pf_company_paise;
  push("5300", expense, 0, "Salary");
  push("5400", 0, pfPayable, "PF payable");
  push("2100", 0, slip.tds_paise, "TDS");
  push("1400", 0, slip.recovery_paise, "Advance recovered");
  push(bank, 0, slip.net_paise, bank === "1000" ? "Cash" : "Bank");
  return lines;
}

export function linesBalance(lines: LedgerLine[]): boolean {
  const dr = lines.reduce((a, l) => a + l.debit_paise, 0);
  const cr = lines.reduce((a, l) => a + l.credit_paise, 0);
  return dr === cr && dr > 0;
}

export type PayrollAsOfRow = {
  id: string;
  employee_id?: string | null;
  employee_name: string;
  effective_from?: string | null;
  revision_no?: number | null;
  status?: string | null;
};

export function pickPayrollAsOf<T extends PayrollAsOfRow>(
  rows: T[],
  person: { employee_id?: string | null; employee_name?: string | null },
  onDate: string,
): T | null {
  const id = (person.employee_id ?? "").trim();
  const name = (person.employee_name ?? "").trim().toLowerCase();
  const date = (onDate || "").slice(0, 10);
  const matches = rows.filter((row) => {
    if (id && row.employee_id === id) return true;
    if (!id && name && row.employee_name.trim().toLowerCase() === name) return true;
    return false;
  });
  const dated = matches
    .filter((row) => !date || !row.effective_from || row.effective_from.slice(0, 10) <= date)
    .sort((a, b) => {
      const da = (a.effective_from ?? "").slice(0, 10);
      const db = (b.effective_from ?? "").slice(0, 10);
      if (da !== db) return db.localeCompare(da);
      return (b.revision_no ?? 0) - (a.revision_no ?? 0);
    });
  return dated[0] ?? null;
}

export function salaryPeriodTaken(
  existing: Array<{
    employee_id?: string | null;
    employee_name?: string;
    pay_kind?: string;
    period_month: number;
    period_year: number;
    id?: string;
  }>,
  input: {
    employee_id?: string | null;
    employee_name: string;
    period_month: number;
    period_year: number;
    id?: string;
  },
): boolean {
  const id = (input.employee_id ?? "").trim();
  const name = input.employee_name.trim().toLowerCase();
  return existing.some((row) => {
    if (normalizePayKind(row.pay_kind) !== "salary") return false;
    if (row.period_month !== input.period_month || row.period_year !== input.period_year) return false;
    if (input.id && row.id === input.id) return false;
    if (id && row.employee_id === id) return true;
    if (!id && (row.employee_name ?? "").trim().toLowerCase() === name) return true;
    return false;
  });
}
