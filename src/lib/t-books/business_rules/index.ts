/**
 * Loopbook logic, ported by hand. Never fetched at runtime.
 * UI must import calculations from here — not reimplement them.
 */

export const LOOPBOOK_LOGIC_VERSION = "v1";

export {
  canonNum,
  conflictMessage,
  sha256Hex,
  voucherCanonical,
  voucherFingerprint,
  type FingerprintFields,
} from "./fingerprint.ts";

export {
  calcPayroll,
  calcPo,
  calcPoItem,
  clampPct,
  paymentTermDays,
  type PoItemCalc,
  type PoItemInput,
  type PoSummary,
  type TermUnit,
} from "./calc-po.ts";
export { formatInr, formatRupees, paiseToRupees, rupeesToPaise } from "./money.ts";
export {
  addPayment,
  amountOrZero,
  canAddPayment,
  emptyPayment,
  MAX_PAYMENT_BLOCKS,
  normalizeVoucherNo,
  occupiedPaymentCount,
  parseAmount,
  parseVoucherNumber,
  summarizeVoucher,
  voucher1001Fixture,
  voucherTotals,
  type PaymentBlock,
  type VoucherComputed,
  type VoucherInput,
} from "./payment.ts";
export {
  isNaTaxInv,
  paymentStatus,
  statusTone,
  taxFlagFor,
  type StatusTone,
  type TaxFlag,
} from "./status.ts";
