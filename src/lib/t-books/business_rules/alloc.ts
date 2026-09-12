/** Hive-side number allocation. Dummy rows never allocate. Posted numbers stay put. */

import { keepPostedPayNumber } from "./hr-payroll.ts";
import { isDummySerial } from "./payment.ts";
import { formatPaymentNumber, parsePayVoucherSeq } from "./purchase-status.ts";

function liveKeys(existing: string[]): string[] {
  return existing.filter((s) => !isDummySerial(s) && s.trim().length > 0);
}

function digitsAfterPrefix(raw: string, prefix: string): number {
  const s = raw.trim();
  const p = prefix.trim();
  if (!p || s.length < p.length) return 0;
  if (s.slice(0, p.length).toUpperCase() !== p.toUpperCase()) return 0;
  const rest = s.slice(p.length);
  const m = rest.match(/^[0-9]+/);
  return m ? Number(m[0]) : 0;
}

/** Shared PUR-NNNN sequence. Also counts legacy PURC:NNNN. */
export function nextPurchaseNumber(existing: string[]): string {
  let max = 0;
  for (const raw of liveKeys(existing)) {
    const s = raw.trim();
    const n = digitsAfterPrefix(s, "PUR-") || digitsAfterPrefix(s, "PURC:");
    if (n > max) max = n;
  }
  return `PUR-${String(max + 1).padStart(4, "0")}`;
}

export function nextConvertedPoNumber(existing: string[]): string {
  return nextPurchaseNumber(existing);
}

export function nextProjectExpenseNumber(existing: string[]): string {
  return nextPurchaseNumber(existing);
}

/** Workspace-wide PAY-0001 series. poNumber is ignored (old call sites). */
export function nextPaymentNumber(existing: string[], _poNumber?: string | null): string {
  let max = 0;
  for (const raw of liveKeys(existing)) {
    max = Math.max(max, parsePayVoucherSeq(raw));
  }
  return formatPaymentNumber(max + 1);
}

export function nextSalaryNumber(existing: string[]): string {
  let max = 0;
  for (const raw of liveKeys(existing)) {
    const n = digitsAfterPrefix(raw.trim(), "SAL-");
    if (n > max) max = n;
  }
  return `SAL-${String(max + 1).padStart(4, "0")}`;
}

/** Next PREFIX-####. Reversed suffixes (PREFIX-0004~id) still count toward max. */
export function allocateVoucherSerial(existingNos: string[], prefix: string): string {
  const p = (prefix ?? "").trim().toUpperCase() || "VCH";
  const needle = `${p}-`;
  let max = 0;
  for (const raw of liveKeys(existingNos)) {
    const n = digitsAfterPrefix(raw.trim(), needle);
    if (n > max) max = n;
  }
  return `${p}-${String(max + 1).padStart(4, "0")}`;
}

export function keepPostedNumber(postedNo: string, requestedNo: string): string {
  return keepPostedPayNumber(postedNo, requestedNo);
}
