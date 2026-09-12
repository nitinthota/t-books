import { emptyPayment, MAX_PAYMENT_BLOCKS, type PaymentBlock } from "./payment.ts";

const FINGERPRINT_VERSION = "v1";

export function canonNum(value: number | null | undefined): string {
  const n = typeof value === "number" && Number.isFinite(value) ? value : 0;
  if (n === 0) return "0";
  const rounded = Math.round(n * 1_000_000) / 1_000_000;
  if (rounded === 0) return "0";
  return String(rounded);
}

function padPayments(payments: PaymentBlock[]): PaymentBlock[] {
  const out: PaymentBlock[] = [];
  for (let i = 1; i <= MAX_PAYMENT_BLOCKS; i++) out.push(emptyPayment(i));
  for (const block of payments.slice(0, MAX_PAYMENT_BLOCKS)) {
    const idx = Math.max(1, Math.min(MAX_PAYMENT_BLOCKS, block.slot || 1)) - 1;
    out[idx] = { ...block, slot: idx + 1 };
  }
  return out;
}

export type FingerprintFields = {
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

export function voucherCanonical(row: FingerprintFields): string {
  const lines: string[] = [
    FINGERPRINT_VERSION,
    String(row.voucher_number),
    row.voucher_date.trim(),
    row.tax_invoice.trim(),
    row.vendor.trim(),
    row.bank.trim(),
    row.account_number.trim(),
    row.ifsc.trim(),
    row.gst.trim(),
    row.project.trim(),
  ];
  for (const block of padPayments(row.payments)) {
    lines.push(`--${block.slot}`);
    lines.push(block.pi_no.trim());
    lines.push(block.pi_date.trim());
    lines.push(canonNum(block.pi_value_rupees));
    lines.push(canonNum(block.payment_percent));
    lines.push(block.description.trim());
    lines.push(canonNum(block.tds_rupees));
    lines.push(canonNum(block.paid_rupees));
    lines.push(block.payment_details.trim());
    lines.push(block.payment_date.trim());
    lines.push(canonNum(block.remaining_rupees));
    lines.push(block.remarks.trim());
  }
  return lines.join("\n");
}

const K = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

function rotr(x: number, n: number): number {
  return (x >>> n) | (x << (32 - n));
}

export function sha256Hex(message: string): string {
  const bytes = new TextEncoder().encode(message);
  const extra = 64 - ((bytes.length + 9) % 64);
  const buf = new Uint8Array(bytes.length + 1 + extra + 8);
  buf.set(bytes);
  buf[bytes.length] = 0x80;
  const bitLen = bytes.length * 8;
  const view = new DataView(buf.buffer);
  view.setUint32(buf.length - 4, bitLen >>> 0, false);
  view.setUint32(buf.length - 8, Math.floor(bitLen / 0x100000000), false);

  let h0 = 0x6a09e667;
  let h1 = 0xbb67ae85;
  let h2 = 0x3c6ef372;
  let h3 = 0xa54ff53a;
  let h4 = 0x510e527f;
  let h5 = 0x9b05688c;
  let h6 = 0x1f83d9ab;
  let h7 = 0x5be0cd19;
  const w = new Uint32Array(64);

  for (let offset = 0; offset < buf.length; offset += 64) {
    for (let i = 0; i < 16; i++) w[i] = view.getUint32(offset + i * 4, false);
    for (let i = 16; i < 64; i++) {
      const s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >>> 3);
      const s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >>> 10);
      w[i] = (w[i - 16] + s0 + w[i - 7] + s1) >>> 0;
    }
    let a = h0;
    let b = h1;
    let c = h2;
    let d = h3;
    let e = h4;
    let f = h5;
    let g = h6;
    let h = h7;
    for (let i = 0; i < 64; i++) {
      const S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
      const ch = (e & f) ^ (~e & g);
      const temp1 = (h + S1 + ch + K[i] + w[i]) >>> 0;
      const S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
      const maj = (a & b) ^ (a & c) ^ (b & c);
      const temp2 = (S0 + maj) >>> 0;
      h = g;
      g = f;
      f = e;
      e = (d + temp1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (temp1 + temp2) >>> 0;
    }
    h0 = (h0 + a) >>> 0;
    h1 = (h1 + b) >>> 0;
    h2 = (h2 + c) >>> 0;
    h3 = (h3 + d) >>> 0;
    h4 = (h4 + e) >>> 0;
    h5 = (h5 + f) >>> 0;
    h6 = (h6 + g) >>> 0;
    h7 = (h7 + h) >>> 0;
  }

  return [h0, h1, h2, h3, h4, h5, h6, h7]
    .map((n) => n.toString(16).padStart(8, "0"))
    .join("");
}

export function voucherFingerprint(row: FingerprintFields): string {
  return sha256Hex(voucherCanonical(row));
}

export function conflictMessage(key: string | number): string {
  const label = typeof key === "number" ? `Voucher ${key}` : key;
  return `${label} was just updated by another user. Reload and submit again.`;
}
