/** Payments stay on the same voucher. At most five PI / payment blocks. */

import { isNaTaxInv, paymentStatus, taxFlagFor, type TaxFlag } from "./status.ts";

export const MAX_PAYMENT_BLOCKS = 5;

export type PaymentBlock = {
  slot: number;
  pi_no: string;
  pi_date: string;
  pi_value_rupees: number | null;
  payment_percent: number | null;
  description: string;
  tds_rupees: number | null;
  paid_rupees: number | null;
  payment_details: string;
  payment_date: string;
  remaining_rupees: number | null;
  remarks: string;
};

export type VoucherInput = {
  voucher_number: number;
  tax_invoice: string;
  vendor: string;
  bank: string;
  account_number: string;
  ifsc: string;
  gst: string;
  project: string;
  comments: string;
  payments: PaymentBlock[];
};

export type VoucherComputed = {
  total_value: number;
  total_paid: number;
  remaining: number;
  status: string;
  tax_flag: TaxFlag;
};

export function emptyPayment(slot: number): PaymentBlock {
  return {
    slot,
    pi_no: "",
    pi_date: "",
    pi_value_rupees: null,
    payment_percent: null,
    description: "",
    tds_rupees: null,
    paid_rupees: null,
    payment_details: "",
    payment_date: "",
    remaining_rupees: null,
    remarks: "",
  };
}

export function parseAmount(raw: string): number | null {
  const cleaned = raw.replace(/[₹,%\s]/g, "").trim();
  if (!cleaned || cleaned === "-" || cleaned === "—") return null;
  const n = Number(cleaned);
  return Number.isFinite(n) ? n : null;
}

export function amountOrZero(raw: string): number {
  return parseAmount(raw) ?? 0;
}

export function normalizeVoucherNo(raw: string): string {
  const s = raw.trim();
  if (/^\d+\.0+$/.test(s)) return String(Number(s));
  return s;
}

export function parseVoucherNumber(raw: string): number | null {
  const n = normalizeVoucherNo(raw);
  if (!/^\d+$/.test(n)) return null;
  return Number(n);
}

function blockOccupied(slot: PaymentBlock): boolean {
  return Boolean(
    slot.pi_value_rupees ||
      slot.paid_rupees ||
      slot.tds_rupees ||
      slot.payment_date ||
      slot.payment_details ||
      slot.pi_no,
  );
}

export function occupiedPaymentCount(blocks: PaymentBlock[]): number {
  return blocks.filter(blockOccupied).length;
}

export function canAddPayment(blocks: PaymentBlock[]): boolean {
  return occupiedPaymentCount(blocks) < MAX_PAYMENT_BLOCKS;
}

export function addPayment(
  blocks: PaymentBlock[],
  next: PaymentBlock,
): { ok: true; payments: PaymentBlock[] } | { ok: false; message: string } {
  if (!canAddPayment(blocks)) {
    return { ok: false, message: "This voucher already has 5 payments." };
  }
  const slot =
    next.slot >= 1 && next.slot <= MAX_PAYMENT_BLOCKS
      ? next.slot
      : occupiedPaymentCount(blocks) + 1;
  return { ok: true, payments: [...blocks, { ...next, slot }] };
}

export function voucherTotals(blocks: PaymentBlock[]): {
  total_value: number;
  total_paid: number;
  remaining: number;
} {
  let invSum = 0;
  let tdsSum = 0;
  let paidSum = 0;
  for (const slot of blocks) {
    invSum += slot.pi_value_rupees ?? 0;
    tdsSum += slot.tds_rupees ?? 0;
    paidSum += slot.paid_rupees ?? 0;
  }
  const payment = paidSum + tdsSum;
  return {
    total_value: invSum,
    total_paid: payment,
    remaining: Math.round(invSum - payment),
  };
}

export function summarizeVoucher(voucher: VoucherInput): VoucherComputed {
  const { total_value, total_paid, remaining } = voucherTotals(voucher.payments);
  const taxForStatus = isNaTaxInv(voucher.tax_invoice) ? "NA" : voucher.tax_invoice;
  const comments =
    voucher.comments.trim() ||
    voucher.payments
      .map((p) => p.remarks)
      .filter(Boolean)
      .join(" · ");
  return {
    total_value,
    total_paid,
    remaining,
    status: paymentStatus(total_value, total_paid, taxForStatus),
    tax_flag: taxFlagFor(voucher.tax_invoice, comments),
  };
}

/** Dummy fixture. Never written to the live books. */
export function voucher1001Fixture(): VoucherInput {
  return {
    voucher_number: 1001,
    tax_invoice: "INV-1001",
    vendor: "VEND_02",
    bank: "BANK_01",
    account_number: "00000000",
    ifsc: "IFSC0000001",
    gst: "GST00DUMMY",
    project: "CUST_01",
    comments: "",
    payments: [
      {
        slot: 1,
        pi_no: "PI-A",
        pi_date: "2026-01-10",
        pi_value_rupees: 60_000,
        payment_percent: 50,
        description: "First block",
        tds_rupees: 2_000,
        paid_rupees: 28_000,
        payment_details: "UTR1",
        payment_date: "2026-01-12",
        remaining_rupees: null,
        remarks: "",
      },
      {
        slot: 2,
        pi_no: "PI-B",
        pi_date: "2026-01-20",
        pi_value_rupees: 40_000,
        payment_percent: 30,
        description: "Second block",
        tds_rupees: null,
        paid_rupees: 12_000,
        payment_details: "UTR2",
        payment_date: "2026-01-22",
        remaining_rupees: null,
        remarks: "",
      },
    ],
  };
}
