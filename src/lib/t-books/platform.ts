import type {
  AccessSnapshot,
  AppStatus,
  AuthInspect,
  RefreshOutcome,
  Session,
  SubmitOutcome,
  VoucherListRow,
  VoucherSummary,
  VoucherView,
} from "./types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function invokeErrorMessage(err: unknown): string {
  if (typeof err === "string" && err.trim()) return err;
  if (err && typeof err === "object" && "message" in err) {
    const message = String((err as { message: unknown }).message ?? "").trim();
    if (message) return message;
  }
  return "That request failed on this PC.";
}

export async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

export async function platformInspect(email: string): Promise<AuthInspect> {
  return invokeCommand<AuthInspect>("inspect", { email });
}

export async function platformLogin(email: string, password: string): Promise<Session> {
  return invokeCommand<Session>("login", { email, password });
}

export async function platformSetPassword(
  email: string,
  password: string,
  confirm: string,
): Promise<Session> {
  return invokeCommand<Session>("set_password", { email, password, confirm });
}

export async function platformLogout(): Promise<void> {
  await invokeCommand<void>("logout");
}

export async function platformRefreshAccess(): Promise<AccessSnapshot> {
  return invokeCommand<AccessSnapshot>("refresh_access");
}

export async function platformGetAccessList(): Promise<AccessSnapshot> {
  return invokeCommand<AccessSnapshot>("get_access_list");
}

export async function platformGetAppStatus(): Promise<AppStatus> {
  return invokeCommand<AppStatus>("get_app_status");
}

export async function platformGetDirtyVouchers(): Promise<number[]> {
  return invokeCommand<number[]>("get_dirty_vouchers");
}

export async function platformRefreshVouchers(): Promise<RefreshOutcome> {
  return invokeCommand<RefreshOutcome>("refresh_vouchers");
}

export async function platformForceRefreshVouchers(): Promise<RefreshOutcome> {
  return invokeCommand<RefreshOutcome>("force_refresh_vouchers");
}

export async function platformGetVoucherSummary(): Promise<VoucherSummary> {
  return invokeCommand<VoucherSummary>("get_voucher_summary");
}

export async function platformListVouchers(): Promise<VoucherListRow[]> {
  return invokeCommand<VoucherListRow[]>("list_vouchers");
}

export async function platformGetVoucher(voucherNumber: number): Promise<VoucherView> {
  return invokeCommand<VoucherView>("get_voucher", { voucherNumber });
}

export async function platformSubmitVoucher(voucherNumber: number): Promise<SubmitOutcome> {
  return invokeCommand<SubmitOutcome>("submit_voucher", { voucherNumber });
}

export async function platformReloadVoucher(voucherNumber: number): Promise<VoucherView> {
  return invokeCommand<VoucherView>("reload_voucher", { voucherNumber });
}

export { invokeErrorMessage };
