import { invokeCommand, isTauriRuntime } from "@/lib/t-books/platform";

export type ArchivedRow = {
  id: number;
  kind: string;
  key: string;
  label: string;
  deletedAt: string;
  purgeAfter: string;
  daysLeft: number;
};

export async function listArchive(): Promise<ArchivedRow[]> {
  if (!isTauriRuntime()) return [];
  return invokeCommand<ArchivedRow[]>("list_archive");
}

export async function restoreArchive(id: number): Promise<void> {
  if (!isTauriRuntime()) return;
  await invokeCommand("restore_archive", { id });
}

export async function deleteVoucher(voucherNumber: number): Promise<void> {
  if (!isTauriRuntime()) return;
  await invokeCommand("delete_voucher", { voucherNumber });
}
