/** PUR-n / PAY-n. Port of loopbook `purchase-status.ts`. */

export type PurchasePayStatus = "Unpaid" | "Partially Paid" | "Fully Paid";
export type PurchaseType = "po" | "non_po";
export type MergeMode = "existing" | "new" | "default";
export type PaymentClass = "advance" | "partial" | "fully_paid";
export type PaymentFilterKey = PaymentClass | "missing_tax";
export type AllocMethod = "advance" | "against" | "on_account";

export type LinkedVoucherRef = { id: string; voucher_no: string };

export const PURCHASE_TYPE_LABELS: Record<PurchaseType, string> = {
  po: "Techsol Purchase",
  non_po: "Project Expenses",
};

export const PAYMENT_CLASS_LABELS: Record<PaymentClass, string> = {
  advance: "Advance",
  partial: "Partial",
  fully_paid: "Fully Paid",
};

export const ALLOC_METHOD_LABELS: Record<AllocMethod, string> = {
  advance: "Advance",
  against: "Against Ref",
  on_account: "On Account",
};

export const PAYMENT_FILTERS: Array<{ key: PaymentFilterKey; label: string }> = [
  { key: "advance", label: "Advance" },
  { key: "partial", label: "Partial" },
  { key: "fully_paid", label: "Fully Paid" },
  { key: "missing_tax", label: "Missing Tax Invoice" },
];

export function normalizePurchaseType(value?: string | null): PurchaseType {
  return value === "non_po" ? "non_po" : "po";
}

export function purchaseTypeLabel(value?: string | null): string {
  return PURCHASE_TYPE_LABELS[normalizePurchaseType(value)];
}

export function purchasePayStatus(grandPaise: number, paidPaise: number): PurchasePayStatus {
  const grand = Number.isFinite(grandPaise) ? grandPaise : 0;
  const paid = Number.isFinite(paidPaise) ? paidPaise : 0;
  if (paid <= 0) return "Unpaid";
  if (paid < grand) return "Partially Paid";
  return "Fully Paid";
}

export function purchaseStatusTone(status: PurchasePayStatus): "credit" | "debit" | "muted" {
  if (status === "Fully Paid") return "credit";
  if (status === "Unpaid") return "debit";
  return "muted";
}

export function normalizePaymentClass(value?: string | null): PaymentClass {
  if (value === "partial" || value === "fully_paid") return value;
  return "advance";
}

export function paymentClassLabel(value?: string | null): string {
  return PAYMENT_CLASS_LABELS[normalizePaymentClass(value)];
}

export function classifyPurchasePayment(input: {
  goodsReceived: boolean;
  taxInvoiceMissing: boolean;
  grandPaise: number;
  paidBeforePaise: number;
  thisPaise: number;
}): { payClass: PaymentClass; missingTaxInvoice: boolean } {
  const grand = Math.max(0, Math.round(Number.isFinite(input.grandPaise) ? input.grandPaise : 0));
  const before = Math.max(0, Math.round(Number.isFinite(input.paidBeforePaise) ? input.paidBeforePaise : 0));
  const amt = Math.max(0, Math.round(Number.isFinite(input.thisPaise) ? input.thisPaise : 0));
  const after = before + amt;
  let payClass: PaymentClass;
  if (!input.goodsReceived) payClass = "advance";
  else if (grand > 0 && after >= grand) payClass = "fully_paid";
  else payClass = "partial";
  return { payClass, missingTaxInvoice: Boolean(input.taxInvoiceMissing) };
}

export function paymentOverTotal(grandPaise: number, paidBeforePaise: number, thisPaise: number): boolean {
  const grand = Math.max(0, Math.round(Number.isFinite(grandPaise) ? grandPaise : 0));
  const before = Math.max(0, Math.round(Number.isFinite(paidBeforePaise) ? paidBeforePaise : 0));
  const amt = Math.max(0, Math.round(Number.isFinite(thisPaise) ? thisPaise : 0));
  if (amt <= 0) return false;
  return before + amt > grand;
}

export function paymentMatchesFilters(
  payment: { pay_class?: string | null; missing_tax_invoice?: boolean },
  selected: PaymentFilterKey[],
): boolean {
  if (!selected.length) return true;
  const cls = normalizePaymentClass(payment.pay_class);
  return selected.some((key) => (key === "missing_tax" ? Boolean(payment.missing_tax_invoice) : cls === key));
}

export function taxInvoiceMissing(no?: string | null, date?: string | null): boolean {
  return !(no ?? "").trim() && !(date ?? "").trim();
}

export function taxInvoiceLabel(no?: string | null, date?: string | null): string {
  const n = (no ?? "").trim();
  const d = (date ?? "").trim().slice(0, 10);
  if (n && d) return `${n} / ${d}`;
  return n || d;
}

export function isHttpUrl(raw: string): boolean {
  const s = raw.trim();
  if (!s) return false;
  try {
    const u = new URL(s);
    return u.protocol === "http:" || u.protocol === "https:";
  } catch {
    return false;
  }
}

export function documentLinkAudit(name: string | null | undefined, userId: string, createdAt: string): string {
  const who = (name ?? "").trim() || (userId ? "team member" : "someone");
  const when = (createdAt ?? "").slice(0, 10);
  return when ? `Added by ${who} · ${when}` : `Added by ${who}`;
}

export function normalizeAllocMethod(value?: string | null): AllocMethod {
  if (value === "against" || value === "on_account") return value;
  return "advance";
}

export function allocMethodForPurchase(goodsReceived: boolean): AllocMethod {
  return goodsReceived ? "against" : "advance";
}

export function allocMethodLabel(value?: string | null): string {
  return ALLOC_METHOD_LABELS[normalizeAllocMethod(value)];
}

export function paymentAllocationLabel(method?: string | null, poNumber?: string | null): string {
  const po = (poNumber ?? "").trim();
  const m = normalizeAllocMethod(method);
  const word = m === "against" ? "Agst Ref" : m === "on_account" ? "On Account" : "Advance";
  return po ? `${word} ${po}` : word;
}

export function parsePayVoucherSeq(raw?: string | null): number {
  const m = /^(?:PAY[-:\s/]+)(\d+)$/i.exec((raw ?? "").trim());
  return m ? Number(m[1]) : 0;
}

export function isPayVoucherNumber(raw?: string | null): boolean {
  return /^PAY-\d+$/i.test((raw ?? "").trim());
}

export function isChildPayNumber(raw?: string | null, poNumber?: string | null): boolean {
  const pay = (raw ?? "").trim();
  if (!pay || isPayVoucherNumber(pay)) return false;
  const po = (poNumber ?? "").trim();
  if (po) {
    const esc = po.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return new RegExp(`^${esc}-(\\d+)$`, "i").test(pay);
  }
  return /^(?!PUR-\d+$)(.+)-(\d{1,2})$/i.test(pay);
}

export function parsePaymentSeq(raw?: string | null, poNumber?: string | null): number {
  const pay = parsePayVoucherSeq(raw);
  if (pay) return pay;
  const child = (raw ?? "").trim();
  const po = (poNumber ?? "").trim();
  if (po) {
    const esc = po.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const tagged = new RegExp(`^${esc}-(\\d+)$`, "i").exec(child);
    if (tagged) return Number(tagged[1]);
  }
  const tail = /^(?!PUR-\d+$)(?!PAY-\d+$)(.+)-(\d{1,2})$/i.exec(child);
  return tail ? Number(tail[2]) : 0;
}

export function formatPaymentNumber(seq: number): string {
  const n = Math.max(1, Math.round(seq));
  return `PAY-${String(n).padStart(4, "0")}`;
}

export function formatPurchasePayNumber(poNumber: string, seq: number): string {
  const po = poNumber.trim() || "PUR";
  const n = Math.max(1, Math.round(seq));
  return `${po}-${String(n).padStart(2, "0")}`;
}

export function isLegacyPayNumber(raw?: string | null): boolean {
  const s = (raw ?? "").trim();
  if (!s) return true;
  if (/^PAY:\d+$/i.test(s)) return true;
  if (/^PAY\s+\d+$/i.test(s)) return true;
  return false;
}

export function paymentOrdinal(seq: number): string {
  const n = Math.max(0, Math.round(seq));
  if (n <= 0) return "payment";
  const mod100 = n % 100;
  const mod10 = n % 10;
  let suffix = "th";
  if (mod100 < 11 || mod100 > 13) {
    if (mod10 === 1) suffix = "st";
    else if (mod10 === 2) suffix = "nd";
    else if (mod10 === 3) suffix = "rd";
  }
  return `${n}${suffix} payment`;
}

export function paymentRef(poNumber?: string | null, payNumber?: string | null, method?: string | null): string {
  const po = (poNumber ?? "").trim();
  const pay = (payNumber ?? "").trim();
  if (pay && po) {
    if (method) return `${pay} · ${paymentAllocationLabel(method, po)}`;
    if (isChildPayNumber(pay, po) || pay.toUpperCase() === po.toUpperCase()) return pay;
    return `${pay} · ${po}`;
  }
  return pay || po;
}

export function isExternalPayment(paidByName?: string | null): boolean {
  return (paidByName ?? "").trim().length > 0;
}

export function vendorMergeKey(vendorName?: string | null, partyId?: string | null): string {
  const name = (vendorName ?? "").trim().toLowerCase();
  if (name) return `name:${name}`;
  const party = (partyId ?? "").trim();
  if (party) return `party:${party}`;
  return "name:";
}

export type MergeVoucherHint = {
  id: string;
  vendor_key: string;
  po_id?: string | null;
  source?: string | null;
  reversed?: boolean;
};

export function mergeBlockedReason(
  vouchers: MergeVoucherHint[],
  targetPoId?: string | null,
  targetVendorKey?: string | null,
): string | null {
  const live = vouchers.filter((v) => !v.reversed);
  if (live.length === 0) return "Select at least one voucher.";
  const keys = new Set(live.map((v) => v.vendor_key));
  if (keys.size > 1) return "Same vendor required.";
  if (targetVendorKey && live[0] && live[0].vendor_key !== targetVendorKey && live[0].vendor_key !== "name:") {
    return "Same vendor required.";
  }
  const linked = [...new Set(live.map((v) => (v.po_id ?? "").trim()).filter(Boolean))];
  if (linked.length > 1) return "A voucher is already on another purchase.";
  if (targetPoId && linked.length === 1 && linked[0] !== targetPoId) {
    return "A voucher is already on another purchase.";
  }
  return null;
}

export function mergeModeBlocked(mode: MergeMode, poId?: string | null): string | null {
  if (mode === "existing" && !(poId ?? "").trim()) return "Choose a purchase to merge onto.";
  return null;
}

export function pickDefaultMergeTarget(input: {
  voucherPoIds: Array<string | null | undefined>;
  vendorPurchaseIds: string[];
}): { kind: "existing"; po_id: string } | { kind: "new" } {
  const linked = [...new Set(input.voucherPoIds.map((id) => (id ?? "").trim()).filter(Boolean))];
  if (linked.length === 1) return { kind: "existing", po_id: linked[0]! };
  if (linked.length === 0 && input.vendorPurchaseIds.length === 1) {
    return { kind: "existing", po_id: input.vendorPurchaseIds[0]! };
  }
  return { kind: "new" };
}

export function isConvertedOrigin(origin?: string | null): boolean {
  return origin === "converted" || origin === "merged";
}

export function isLinkedImportPayment(source?: string | null): boolean {
  const s = (source ?? "").trim();
  return Boolean(s) && s !== "purchase";
}

export function canUnmergeVoucher(source?: string | null): boolean {
  return isLinkedImportPayment(source);
}

export function deleteReasonOk(reason?: string | null): boolean {
  return (reason ?? "").trim().length >= 3;
}

export function isBlankPoItem(item: {
  item_name?: string | null;
  description?: string | null;
  qty?: number | null;
  unit_rate_rupees?: number | null;
}): boolean {
  const named = Boolean((item.item_name ?? "").trim() || (item.description ?? "").trim());
  const valued = Number(item.unit_rate_rupees) > 0;
  return !named && !valued;
}

export function displayPurchaseGrandPaise(previewGrandPaise: number, storedGrandPaise: number): number {
  const preview = Number.isFinite(previewGrandPaise) ? previewGrandPaise : 0;
  const stored = Number.isFinite(storedGrandPaise) ? storedGrandPaise : 0;
  if (preview > 0) return preview;
  return stored > 0 ? stored : 0;
}

export function poListMoney(input: {
  kind: string;
  grand_total_paise: number;
  amount_received_paise: number;
  paid_paise: number;
  balance_paise: number;
}): { amount_received_paise: number; paid_paise: number; balance_paise: number } {
  const paid = Number(input.paid_paise) || 0;
  const grand = Number(input.grand_total_paise) || 0;
  if (input.kind === "purchase") {
    return {
      amount_received_paise: paid,
      paid_paise: paid,
      balance_paise: Number.isFinite(input.balance_paise) ? input.balance_paise : grand - paid,
    };
  }
  const received = Number(input.amount_received_paise) || 0;
  return {
    amount_received_paise: received,
    paid_paise: paid,
    balance_paise: grand - received,
  };
}

export function sumRupees(values: Array<number | null | undefined>): number {
  return values.reduce<number>((a, v) => a + (Number(v) || 0), 0);
}

export function combineVoucherDescriptions(
  existing?: string | null,
  narrations: Array<string | null | undefined> = [],
): string {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of [existing, ...narrations]) {
    const t = (raw ?? "").trim();
    if (!t) continue;
    const key = t.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(t);
  }
  return out.join(" · ");
}

export function shouldPostPurchaseBill(input: {
  goodsReceived: boolean;
  grandPaise: number;
  converted?: boolean;
}): boolean {
  if (input.converted) return false;
  if (!(input.grandPaise > 0)) return false;
  return input.goodsReceived === true;
}

export function lastVoucherUnmerge(
  remainingCount: number,
  opts: { autoDelete?: boolean; itemCount?: number; hasPostedPayments?: boolean },
): "keep" | "delete" | "empty" {
  if (remainingCount > 0) return "keep";
  if (opts.hasPostedPayments || (opts.itemCount ?? 0) > 0) return "empty";
  if (opts.autoDelete) return "delete";
  return "empty";
}

export function formatBankLabel(
  bank?: { bank_name?: string | null; account_no?: string | null; ifsc?: string | null; holder_name?: string | null } | null,
): string {
  if (!bank) return "";
  return [bank.bank_name, bank.account_no, bank.ifsc, bank.holder_name]
    .map((s) => (s ?? "").trim())
    .filter(Boolean)
    .join(" · ");
}

export function hasBankDetails(
  bank?: { bank_name?: string | null; account_no?: string | null; ifsc?: string | null } | null,
): boolean {
  if (!bank) return false;
  return Boolean((bank.bank_name ?? "").trim() || (bank.account_no ?? "").trim() || (bank.ifsc ?? "").trim());
}

export function parseVoucherLinks(raw?: string | null): LinkedVoucherRef[] {
  const text = (raw ?? "").trim();
  if (!text) return [];
  if (text.startsWith("[") || text.startsWith("{")) {
    try {
      const parsed = JSON.parse(text) as unknown;
      return (Array.isArray(parsed) ? parsed : [])
        .map((row) => {
          if (!row || typeof row !== "object") return null;
          const rec = row as { id?: unknown; voucher_no?: unknown };
          const id = String(rec.id ?? "").trim();
          const voucher_no = String(rec.voucher_no ?? "").trim();
          if (!voucher_no) return null;
          return { id, voucher_no };
        })
        .filter((x): x is LinkedVoucherRef => Boolean(x));
    } catch {
      /* fall through */
    }
  }
  return text
    .split(/\n+/)
    .map((line) => {
      const tab = line.indexOf("\t");
      if (tab < 0) {
        const no = line.trim();
        return no ? { id: "", voucher_no: no } : null;
      }
      const id = line.slice(0, tab).trim();
      const voucher_no = line.slice(tab + 1).trim();
      return voucher_no ? { id, voucher_no } : null;
    })
    .filter((x): x is LinkedVoucherRef => Boolean(x));
}
