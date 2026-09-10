/** Status from amounts. Port of loopbook paymentStatus / taxFlagFor. */

export type TaxFlag = "missing" | "na_needs_comment" | "na_ok" | "ok";
export type StatusTone = "credit" | "debit" | "muted";

export function isNaTaxInv(raw: string): boolean {
  return /^(na|n\/a)$/i.test(raw.trim());
}

export function paymentStatus(invoice: number, payment: number, taxInv: string): string {
  const out = Math.round(invoice - payment);
  if (out < 0) return "Advance Payment";
  if (!taxInv.trim()) return "Missing Tax Invoice";
  if (payment === 0) return "Pending Payment";
  if (out === 0) return "Full Payment";
  return "Partial Payment";
}

export function taxFlagFor(taxInv: string, comments: string): TaxFlag {
  const t = taxInv.trim();
  if (!t) return "missing";
  if (isNaTaxInv(t)) return comments.trim() ? "na_ok" : "na_needs_comment";
  return "ok";
}

export function statusTone(status: string): StatusTone {
  const s = status.toLowerCase();
  if (s.includes("full") || s.includes("advance")) return "credit";
  if (s.includes("pending") || s.includes("missing") || s.includes("partial")) return "debit";
  return "muted";
}
