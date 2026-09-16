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
} from "./po-calc.ts";

export {
  clampAsOf,
  currentIndianFy,
  fyBounds,
  fyOptions,
  fyRange,
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
  allocMethodLabel,
  canUnmergeVoucher,
  classifyPurchasePayment,
  deleteReasonOk,
  formatBankLabel,
  formatPaymentNumber,
  isBlankPoItem,
  isChildPayNumber,
  isHttpUrl,
  isLegacyPayNumber,
  isPayVoucherNumber,
  lastVoucherUnmerge,
  mergeBlockedReason,
  normalizeAllocMethod,
  parsePayVoucherSeq,
  paymentMatchesFilters,
  paymentOverTotal,
  pickDefaultMergeTarget,
  poListMoney,
  purchasePayStatus,
  shouldPostPurchaseBill,
  taxInvoiceMissing,
  vendorMergeKey,
  type AllocMethod,
  type MergeVoucherHint,
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

export {
  allocateVoucherSerial as allocateVoucherSerialFromSheet,
  countSheetPaymentLines,
  headerMoneyFromLines,
  parseVoucherDataWorkbook,
  VOUCHER_RAW_SHEET_NAME,
} from "./voucher-data.ts";
