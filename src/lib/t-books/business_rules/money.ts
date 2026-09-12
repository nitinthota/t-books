/** Paise in store, rupees on screen. Port of loopbook money.ts. */

export function rupeesToPaise(rupees: number): number {
  const n = Number(rupees);
  if (!Number.isFinite(n)) return 0;
  return Math.round(n * 100);
}

export function paiseToRupees(paise: number): number {
  return paise / 100;
}

export function formatInr(paise: number): string {
  const sign = paise < 0 ? "-" : "";
  const abs = Math.abs(paise);
  const rs = Math.floor(abs / 100);
  const ps = abs % 100;
  const grouped = rs.toLocaleString("en-IN");
  return `${sign}₹${grouped}.${String(ps).padStart(2, "0")}`;
}

export function formatRupees(rupees: number | null | undefined): string {
  if (rupees == null || !Number.isFinite(rupees)) return "—";
  const sign = rupees < 0 ? "-" : "";
  return `${sign}₹${Math.abs(rupees).toLocaleString("en-IN", {
    maximumFractionDigits: 2,
    minimumFractionDigits: 0,
  })}`;
}

export function formatQty(qty: number | string): string {
  const n = typeof qty === "string" ? Number(qty) : qty;
  if (!Number.isFinite(n)) return "—";
  return n.toLocaleString("en-IN", { maximumFractionDigits: 3 });
}
