/** Indian financial year: 1 Apr–31 Mar. Dummy dates only in tests. */

export function currentIndianFy(today = new Date()): string {
  const y = today.getUTCFullYear();
  const m = today.getUTCMonth();
  const startYear = m >= 3 ? y : y - 1;
  return `${startYear}-${String((startYear + 1) % 100).padStart(2, "0")}`;
}

export function fyBounds(fy: string): { start: string; end: string } {
  const startYear = Number(String(fy).slice(0, 4));
  if (!Number.isFinite(startYear) || startYear < 1990 || startYear > 2100) {
    return fyBounds(currentIndianFy());
  }
  const endYear = startYear + 1;
  return { start: `${startYear}-04-01`, end: `${endYear}-03-31` };
}

/** Loopbook name lock — same as fyBounds. */
export function fyRange(fy: string): { start: string; end: string } {
  return fyBounds(fy);
}

export function fyOptions(minDate: string | null, maxDate: string | null, today = new Date()): string[] {
  const years = new Set<number>();
  const pushDate = (iso: string | null) => {
    if (!iso) return;
    const d = new Date(`${iso.slice(0, 10)}T00:00:00Z`);
    if (Number.isNaN(d.getTime())) return;
    years.add(d.getUTCMonth() >= 3 ? d.getUTCFullYear() : d.getUTCFullYear() - 1);
  };
  pushDate(minDate);
  pushDate(maxDate);
  const cur = currentIndianFy(today);
  years.add(Number(cur.slice(0, 4)));
  return [...years]
    .filter((y) => Number.isFinite(y))
    .sort((a, b) => b - a)
    .map((y) => `${y}-${String((y + 1) % 100).padStart(2, "0")}`);
}

export function clampAsOf(asOf: string | undefined, start: string, end: string, todayIso: string): string {
  const raw = (asOf || todayIso).slice(0, 10);
  if (raw < start) return start;
  if (raw > end) return end;
  return raw;
}

export function trialClosing(openingPaise: number, periodDr: number, periodCr: number): number {
  return openingPaise + periodDr - periodCr;
}

export function signedDrCr(paise: number): { debit_paise: number; credit_paise: number } {
  if (paise > 0) return { debit_paise: paise, credit_paise: 0 };
  if (paise < 0) return { debit_paise: 0, credit_paise: -paise };
  return { debit_paise: 0, credit_paise: 0 };
}

export function trialBalanced(rows: Array<{ debit_paise: number; credit_paise: number }>): boolean {
  const dr = rows.reduce((s, r) => s + r.debit_paise, 0);
  const cr = rows.reduce((s, r) => s + r.credit_paise, 0);
  return dr === cr;
}
