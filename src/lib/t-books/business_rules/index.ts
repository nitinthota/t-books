/**
 * Loopbook logic, ported by hand. Never fetched at runtime.
 * UI must import calculations from here — not reimplement them.
 */

export const LOOPBOOK_LOGIC_VERSION = "v1";

export {
  allocateOrReject,
  allocateVoucherSerial,
  keepPostedNumber,
  mergeKeyPool,
  nextConvertedPoNumber,
  nextPaymentFromHiveAndLocal,
  nextPaymentNumber,
  nextProjectExpenseNumber,
  nextPurchaseFromHiveAndLocal,
  nextPurchaseNumber,
  nextSalaryFromHiveAndLocal,
  nextSalaryNumber,
} from "./alloc.ts";

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

export {
  clampAsOf,
  currentIndianFy,
  fyBounds,
  fyOptions,
  signedDrCr,
  trialBalanced,
  trialClosing,
} from "./fy.ts";

export {
  bankAccountCode,
  calcSalarySlip,
  keepPostedPayNumber,
  linesBalance,
  monthLabel,
  normalizePayKind,
  outstandingAdvancePaise,
  payrollMoneyChanged,
  payrollNetOk,
  pickPayrollAsOf,
  salaryPeriodTaken,
  salaryVoucherLines,
  type LedgerLine,
  type PayKind,
  type PayrollMoney,
  type SalarySlip,
  MONTHS,
} from "./hr-payroll.ts";

export { formatInr, formatQty, formatRupees, paiseToRupees, rupeesToPaise } from "./money.ts";

export {
  addPayment,
  amountOrZero,
  canAddPayment,
  emptyPayment,
  isDottedChildSerial,
  isDummySerial,
  MAX_PAYMENT_BLOCKS,
  normalizeVoucherNo,
  occupiedPaymentCount,
  parseAmount,
  parseVoucherNumber,
  shouldSkipSheetRow,
  summarizeVoucher,
  voucher1001Fixture,
  voucherTotals,
  type PaymentBlock,
  type VoucherComputed,
  type VoucherInput,
} from "./payment.ts";

export {
  allocMethodForPurchase,
  classifyPurchasePayment,
  formatPaymentNumber,
  isChildPayNumber,
  isLegacyPayNumber,
  isPayVoucherNumber,
  normalizeAllocMethod,
  parsePayVoucherSeq,
  paymentMatchesFilters,
  paymentOverTotal,
  purchasePayStatus,
  shouldPostPurchaseBill,
  taxInvoiceMissing,
  type AllocMethod,
  type PaymentClass,
  type PaymentFilterKey,
  type PurchasePayStatus,
  type PurchaseType,
} from "./purchase-status.ts";

export {
  canMutate,
  canOpenAccessRole,
  canRefreshRole,
  canWrite,
  requireAdmin,
  requireWrite,
} from "./rbac.ts";

export {
  isNaTaxInv,
  paymentStatus,
  statusTone,
  taxFlagFor,
  type StatusTone,
  type TaxFlag,
} from "./status.ts";
