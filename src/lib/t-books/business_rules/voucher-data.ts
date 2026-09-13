/** Canonical register schema for Voucher_Data. Patterns only — never raw cells. */

export const VOUCHER_RAW_SHEET_NAME = "Voucher_Raw_Data";

export const VOUCHER_HEADER_COLUMNS = [
  "serial_no",
  "voucher_date",
  "tax_inv_no_date",
  "vendor_name",
  "bank_name",
  "account_no",
  "ifsc",
  "gst_no",
  "project_name",
  "bill_amount",
  "status",
] as const;

const HEADER_COLS = 9;
const SLOT_WIDTH = 11;
const SLOT_COUNT = 5;

export type PaymentSlot = {
  pay_date: string;
  pay_mode: string;
  cheque_no: string;
  cheque_date: string;
  debit_ac: string;
  amount_rupees: number | null;
  tds_percent: number | null;
  tds_amount_rupees: number | null;
  net_paid_rupees: number | null;
  utr_ref: string;
  pi_value_rupees: number | null;
  remaining_rupees: number | null;
  description: string;
  remarks: string;
};

export type TaxFlag = "missing" | "na_needs_comment" | "na_ok" | "ok";

export type SheetVoucher = {
  id: string;
  row: number;
  serial: string;
  voucher_date: string;
  tax_inv: string;
  vendor_name: string;
  project: string;
  gst_no: string;
  bank_name: string;
  account_no: string;
  ifsc: string;
  bill_amount: number | null;
  status: string;
  tax_flag: TaxFlag;
  comments: string;
  description: string;
  payments: PaymentSlot[];
  paid_total: number;
  outstanding: number;
};

export type SheetReport = {
  total_invoice: number;
  payment_made: number;
  outstanding: number;
  full_payment: number;
  partial_payment: number;
  advance_payment: number;
  pending_payment: number;
  missing_tax_invoice: number;
  na_needs_comment: number;
  skipped_empty: number;
};

export function emptyPayment(): PaymentSlot {
  return {
    pay_date: "",
    pay_mode: "",
    cheque_no: "",
    cheque_date: "",
    debit_ac: "",
    amount_rupees: null,
    tds_percent: null,
    tds_amount_rupees: null,
    net_paid_rupees: null,
    utr_ref: "",
    pi_value_rupees: null,
    remaining_rupees: null,
    description: "",
    remarks: "",
  };
}

export function parseAmount(raw: string): number | null {
  const cleaned = raw.replace(/[₹,%,\s]/g, "").trim();
  if (!cleaned || cleaned === "-" || cleaned === "\u2014") return null;
  const n = Number(cleaned);
  return Number.isFinite(n) ? n : null;
}

function splitCsvLine(line: string): string[] {
  const out: string[] = [];
  let cur = "";
  let q = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (q) {
      if (ch === '"') {
        if (line[i + 1] === '"') {
          cur += '"';
          i++;
        } else q = false;
      } else cur += ch;
    } else if (ch === '"') q = true;
    else if (ch === ",") {
      out.push(cur);
      cur = "";
    } else cur += ch;
  }
  out.push(cur);
  return out;
}

function cell(row: string[], i: number): string {
  return (row[i] ?? "").replace(/\s+/g, " ").trim();
}

export function normalizeVoucherNo(raw: string): string {
  const s = raw.trim();
  if (/^\d+\.0+$/.test(s)) return String(Number(s));
  return s;
}

export function isDottedChildSerial(raw: string): boolean {
  return /^\d+\.(?!0+$)\d+$/.test(raw.trim());
}

export function parseSheetDate(raw: string): string | null {
  const s = (raw || "").trim();
  if (!s) return null;
  const iso = s.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (iso) return `${iso[1]}-${iso[2]}-${iso[3]}`;
  const dmy = s.match(/^(\d{1,2})[.\/-](\d{1,2})[.\/-](\d{2,4})$/);
  if (dmy) {
    const y = dmy[3].length === 2 ? `20${dmy[3]}` : dmy[3];
    const dd = dmy[1].padStart(2, "0");
    const mm = dmy[2].padStart(2, "0");
    if (Number(mm) >= 1 && Number(mm) <= 12 && Number(dd) >= 1 && Number(dd) <= 31) {
      return `${y}-${mm}-${dd}`;
    }
  }
  return null;
}

export function headerMoneyFromLines(
  lines: Array<{
    remarks?: string | null;
    debit_paise?: number | null;
    amount_rupees?: number | null;
    tds_amount_rupees?: number | null;
  }>,
  header: { invoice_rupees: number; paid_rupees: number },
): { invoice_rupees: number; paid_rupees: number } {
  const bills = lines.filter((l) => /^bill/i.test((l.remarks ?? "").trim()));
  const pays = lines.filter((l) => /^payment/i.test((l.remarks ?? "").trim()));
  const invoice = bills.length
    ? bills.reduce((s, l) => s + (Number(l.amount_rupees) || (l.debit_paise ?? 0) / 100), 0)
    : header.invoice_rupees;
  const paid = pays.length
    ? pays.reduce((s, l) => {
        const amt = Number(l.amount_rupees) || 0;
        const tds = Number(l.tds_amount_rupees) || 0;
        if (amt + tds > 0) return s + amt + tds;
        return s + (l.debit_paise ?? 0) / 100;
      }, 0)
    : header.paid_rupees;
  return { invoice_rupees: invoice, paid_rupees: paid };
}

export function allocateVoucherSerial(existingNos: string[], prefix: string): string {
  const p = (prefix ?? "").trim().toUpperCase() || "VCH";
  const escaped = p.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`^${escaped}-([0-9]+)`, "i");
  let max = 0;
  for (const raw of existingNos) {
    const m = re.exec((raw ?? "").trim());
    if (m) max = Math.max(max, Number(m[1]));
  }
  return `${p}-${String(max + 1).padStart(4, "0")}`;
}

function slotFrom(row: string[], start: number): PaymentSlot {
  const paid = parseAmount(cell(row, start + 6));
  const tds = parseAmount(cell(row, start + 5));
  return {
    cheque_no: cell(row, start),
    cheque_date: cell(row, start + 1),
    pi_value_rupees: parseAmount(cell(row, start + 2)),
    description: cell(row, start + 4),
    pay_mode: cell(row, start + 4),
    tds_percent: null,
    tds_amount_rupees: tds != null && tds > 0 ? tds : null,
    amount_rupees: paid,
    net_paid_rupees: paid,
    utr_ref: cell(row, start + 7),
    pay_date: cell(row, start + 8),
    remaining_rupees: parseAmount(cell(row, start + 9)),
    remarks: cell(row, start + 10),
    debit_ac: "",
  };
}

export function paymentStatus(invoice: number, payment: number, taxInv: string): string {
  const outPaise = Math.round(invoice * 100) - Math.round(payment * 100);
  if (outPaise < 0) return "Advance Payment";
  if (!taxInv.trim()) return "Missing Tax Invoice";
  if (Math.round(payment * 100) === 0) return "Pending Payment";
  if (outPaise === 0) return "Full Payment";
  return "Partial Payment";
}

export function countSheetPaymentLines(
  lines: { remarks: string; tds_amount_rupees?: number }[],
): { payments: number; tds: number } {
  let payments = 0;
  let tds = 0;
  for (const l of lines) {
    const remarks = l.remarks ?? "";
    if (/^payment/i.test(remarks)) payments += 1;
    if (/^tds/i.test(remarks) || Number(l.tds_amount_rupees) > 0) tds += 1;
  }
  return { payments, tds };
}

export function isNaTaxInv(raw: string): boolean {
  return /^(na|n\/a)$/i.test(raw.trim());
}

export function taxFlagFor(taxInv: string, comments: string): TaxFlag {
  const t = taxInv.trim();
  if (!t) return "missing";
  if (isNaTaxInv(t)) return comments.trim() ? "na_ok" : "na_needs_comment";
  return "ok";
}

export function statusTone(status: string): "credit" | "debit" | "muted" {
  const s = status.toLowerCase();
  if (s.includes("full")) return "credit";
  if (s.includes("advance")) return "credit";
  if (s.includes("pending") || s.includes("missing") || s.includes("partial")) return "debit";
  return "muted";
}

export function extractSheetCsv(csv: string, sheetName = VOUCHER_RAW_SHEET_NAME): string | null {
  const wanted = sheetName.trim().toLowerCase();
  const re = /--- Sheet: (.*?) ---/g;
  const hits: Array<{ name: string; start: number; marker: number }> = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(csv))) {
    hits.push({ name: m[1].trim(), start: m.index + m[0].length, marker: m.index });
  }
  if (hits.length === 0) return csv;
  const bodies = hits.map((h, i) => ({
    name: h.name,
    body: csv.slice(h.start, i + 1 < hits.length ? hits[i + 1].marker : csv.length),
  }));
  const match = bodies.find((b) => b.name.trim().toLowerCase() === wanted);
  return match ? match.body : null;
}

function isHeaderRow(cols: string[]): boolean {
  const head = cols.slice(0, 9).join(" ").toLowerCase();
  if (head.includes("vendor name") || head.includes("tax inv") || head.includes("voucher date")) return true;
  if (cell(cols, 0).toUpperCase() === "F" && head.includes("project")) return true;
  return false;
}

export function parseVoucherDataWorkbook(csv: string): {
  vouchers: SheetVoucher[];
  report: SheetReport;
} {
  const report: SheetReport = {
    total_invoice: 0,
    payment_made: 0,
    outstanding: 0,
    full_payment: 0,
    partial_payment: 0,
    advance_payment: 0,
    pending_payment: 0,
    missing_tax_invoice: 0,
    na_needs_comment: 0,
    skipped_empty: 0,
  };
  const primary = extractSheetCsv(csv, VOUCHER_RAW_SHEET_NAME);
  if (primary == null) return { vouchers: [], report };
  const lines = primary.split(/\r?\n/).filter((l) => l.trim().length > 0);
  const vouchers: SheetVoucher[] = [];
  for (let i = 0; i < lines.length; i++) {
    const cols = splitCsvLine(lines[i]);
    if (cols.length < HEADER_COLS) continue;
    if (isHeaderRow(cols)) continue;
    const serial = normalizeVoucherNo(cell(cols, 0));
    const vendor = cell(cols, 3);
    if (!vendor || isDottedChildSerial(serial)) {
      report.skipped_empty += 1;
      continue;
    }
    if (serial && !/^\d+$/.test(serial) && serial.length > 12) {
      report.skipped_empty += 1;
      continue;
    }
    const payments: PaymentSlot[] = [];
    let invSum = 0;
    let tdsSum = 0;
    let paidSum = 0;
    for (let s = 0; s < SLOT_COUNT; s++) {
      const start = HEADER_COLS + s * SLOT_WIDTH;
      if (start >= cols.length) break;
      const slot = slotFrom(cols, start);
      invSum += slot.pi_value_rupees ?? 0;
      tdsSum += slot.tds_amount_rupees ?? 0;
      paidSum += slot.amount_rupees ?? 0;
      if (slot.pi_value_rupees || slot.amount_rupees || slot.tds_amount_rupees || slot.pay_date || slot.utr_ref || slot.cheque_no) {
        payments.push(slot);
      }
    }
    const payment = paidSum + tdsSum;
    const outstanding = Math.round(invSum - payment);
    const taxInv = cell(cols, 2);
    const comments = payments.map((p) => p.remarks).filter(Boolean).join(" \u00b7 ");
    const description = payments.map((p) => p.description).filter(Boolean).join(" \u00b7 ");
    const status = paymentStatus(invSum, payment, isNaTaxInv(taxInv) ? "NA" : taxInv);
    const tax_flag = taxFlagFor(taxInv, comments);
    const id = serial || `ROW_${i + 1}`;
    vouchers.push({
      id,
      row: i + 1,
      serial,
      voucher_date: cell(cols, 1),
      tax_inv: taxInv,
      vendor_name: vendor,
      project: cell(cols, 8),
      gst_no: cell(cols, 7),
      bank_name: cell(cols, 4),
      account_no: cell(cols, 5),
      ifsc: cell(cols, 6),
      bill_amount: invSum,
      status,
      tax_flag,
      comments,
      description,
      payments,
      paid_total: payment,
      outstanding,
    });
    report.total_invoice += invSum;
    report.payment_made += payment;
    report.outstanding += outstanding;
    if (tax_flag === "na_needs_comment") report.na_needs_comment += 1;
    const sl = status.toLowerCase();
    if (sl.includes("missing")) report.missing_tax_invoice += 1;
    else if (sl.includes("pending")) report.pending_payment += 1;
    else if (sl.includes("partial")) report.partial_payment += 1;
    else if (sl.includes("advance")) report.advance_payment += 1;
    else if (sl.includes("full")) report.full_payment += 1;
  }
  return { vouchers, report };
}
