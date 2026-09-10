import {
  amountOrZero,
  emptyPayment,
  MAX_PAYMENT_BLOCKS,
  parseVoucherNumber,
  type PaymentBlock,
} from "./business_rules/index.ts";
import type { RowError } from "./types.ts";

/** A=voucher_number, B=voucher_date, C–I register, then 5×11 payment blocks. */
const HEADER_COLS = 9;
const SLOT_WIDTH = 11;

export type ParsedVoucher = {
  voucher_number: number;
  voucher_date: string;
  tax_invoice: string;
  vendor: string;
  bank: string;
  account_number: string;
  ifsc: string;
  gst: string;
  project: string;
  payments: PaymentBlock[];
};

export type ParseReport = {
  vouchers: ParsedVoucher[];
  errors: RowError[];
  skipped: number;
};

function cell(row: string[], i: number): string {
  return row[i] ?? "";
}

function isHeaderRow(cols: string[]): boolean {
  const head = cols.slice(0, HEADER_COLS).join(" ").toLowerCase();
  if (
    head.includes("vendor name") ||
    head.includes("tax inv") ||
    head.includes("voucher date") ||
    head.includes("voucher_number") ||
    head.includes("serial")
  ) {
    return true;
  }
  const a = cell(cols, 0).trim().toLowerCase();
  return a === "f" || a === "voucher_number" || a === "voucher number";
}

function slotOccupied(slot: PaymentBlock): boolean {
  return Boolean(
    (slot.pi_value_rupees ?? 0) !== 0 ||
      (slot.paid_rupees ?? 0) !== 0 ||
      (slot.tds_rupees ?? 0) !== 0 ||
      slot.payment_date ||
      slot.payment_details ||
      slot.pi_no,
  );
}

function slotFrom(row: string[], start: number, slot: number): PaymentBlock {
  const block = emptyPayment(slot);
  block.pi_no = cell(row, start).trim();
  block.pi_date = cell(row, start + 1).trim();
  block.pi_value_rupees = amountOrZero(cell(row, start + 2));
  block.payment_percent = amountOrZero(cell(row, start + 3));
  block.description = cell(row, start + 4).trim();
  block.tds_rupees = amountOrZero(cell(row, start + 5));
  block.paid_rupees = amountOrZero(cell(row, start + 6));
  block.payment_details = cell(row, start + 7).trim();
  block.payment_date = cell(row, start + 8).trim();
  block.remaining_rupees = amountOrZero(cell(row, start + 9));
  block.remarks = cell(row, start + 10).trim();
  return block;
}

function logSkip(row: number, voucherNumber: number | null, errorType: string): void {
  if (voucherNumber == null) {
    console.info(`t-books refresh skip row=${row} voucher=none type=${errorType}`);
  } else {
    console.info(`t-books refresh skip row=${row} voucher=${voucherNumber} type=${errorType}`);
  }
}

export function parseVoucherValues(values: string[][]): ParseReport {
  const errors: RowError[] = [];
  let skipped = 0;
  const byNumber = new Map<number, ParsedVoucher>();

  for (let i = 0; i < values.length; i++) {
    const cols = values[i] ?? [];
    const row = i + 1;
    if (cols.every((c) => !c.trim())) continue;
    if (isHeaderRow(cols)) continue;
    const voucherNumber = parseVoucherNumber(cell(cols, 0));
    if (voucherNumber == null) {
      skipped += 1;
      logSkip(row, null, "invalid_voucher_number");
      errors.push({ voucherNumber: null, row, errorType: "invalid_voucher_number" });
      continue;
    }
    const payments: PaymentBlock[] = [];
    for (let s = 0; s < MAX_PAYMENT_BLOCKS; s++) {
      const start = HEADER_COLS + s * SLOT_WIDTH;
      if (start >= cols.length) break;
      const slot = slotFrom(cols, start, s + 1);
      if (slotOccupied(slot)) payments.push(slot);
    }
    byNumber.set(voucherNumber, {
      voucher_number: voucherNumber,
      voucher_date: cell(cols, 1).trim(),
      tax_invoice: cell(cols, 2).trim(),
      vendor: cell(cols, 3).trim(),
      bank: cell(cols, 4).trim(),
      account_number: cell(cols, 5).trim(),
      ifsc: cell(cols, 6).trim(),
      gst: cell(cols, 7).trim(),
      project: cell(cols, 8).trim(),
      payments,
    });
  }

  return { vouchers: [...byNumber.values()], errors, skipped };
}
