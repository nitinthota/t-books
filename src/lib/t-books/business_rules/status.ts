/** Status from amounts. One source: voucher-data.ts (Loopbook file name). */

export {
  isNaTaxInv,
  paymentStatus,
  statusTone,
  taxFlagFor,
  type TaxFlag,
} from "./voucher-data.ts";

export type StatusTone = "credit" | "debit" | "muted";
