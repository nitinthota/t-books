import { exec, withTransaction } from "./db";

export function discardDirtyKey(kind: string, key: string): void {
  const k = key.trim();
  if (!k) return;
  withTransaction(() => {
    if (kind === "voucher") {
      const n = Number(k);
      if (!Number.isFinite(n)) return;
      exec("DELETE FROM voucher_payments WHERE voucher_number = ?", [n]);
      exec("DELETE FROM vouchers WHERE voucher_number = ?", [n]);
    } else if (kind === "purchase") {
      exec("DELETE FROM purchase_po WHERE po_number = ? COLLATE NOCASE", [k]);
    } else if (kind === "payment") {
      exec("DELETE FROM purchase_payments WHERE pay_number = ? COLLATE NOCASE", [k]);
    } else if (kind === "salary") {
      exec("DELETE FROM hr_payroll WHERE salary_number = ? COLLATE NOCASE", [k]);
    } else if (kind === "inventory") {
      const id = Number(k.replace(/^INV-/i, ""));
      if (Number.isFinite(id)) exec("DELETE FROM inventory WHERE id = ?", [id]);
    } else if (kind === "logistics") {
      const id = Number(k.replace(/^TRIP-/i, ""));
      if (Number.isFinite(id)) exec("DELETE FROM logistics WHERE id = ?", [id]);
    } else if (kind === "document") {
      const id = Number(k.replace(/^DOC-/i, ""));
      if (Number.isFinite(id)) exec("DELETE FROM documents WHERE id = ?", [id]);
    }
    try {
      exec("DELETE FROM pending_submit WHERE kind = ? AND key = ?", [kind, k]);
    } catch {
      /* table may be missing */
    }
  });
}

export function discardAllDirty(): void {
  withTransaction(() => {
    exec("DELETE FROM voucher_payments WHERE voucher_number IN (SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1)");
    exec("DELETE FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1");
    try { exec("DELETE FROM purchase_po WHERE COALESCE(is_dirty,0) = 1"); } catch { /* */ }
    try { exec("DELETE FROM purchase_payments WHERE COALESCE(is_dirty,0) = 1"); } catch { /* */ }
    try { exec("DELETE FROM hr_payroll WHERE COALESCE(is_dirty,0) = 1"); } catch { /* */ }
    try { exec("DELETE FROM inventory WHERE COALESCE(is_dirty,0) = 1"); } catch { /* */ }
    try { exec("DELETE FROM logistics WHERE COALESCE(is_dirty,0) = 1"); } catch { /* */ }
    try { exec("DELETE FROM documents WHERE COALESCE(is_dirty,0) = 1"); } catch { /* */ }
    try { exec("DELETE FROM pending_submit"); } catch { /* */ }
  });
}
