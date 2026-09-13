import { invokeCommand, isTauriRuntime } from "./platform";
import type { VoucherSave, VoucherView } from "./types";

export async function saveVoucher(payload: VoucherSave): Promise<VoucherView> {
  if (isTauriRuntime()) {
    return invokeCommand<VoucherView>("save_voucher", { payload });
  }
  throw new Error("Save voucher requires the T Books desktop app.");
}

export async function nextVoucherNumber(): Promise<number> {
  if (isTauriRuntime()) {
    return invokeCommand<number>("next_voucher_number");
  }
  return 1;
}
