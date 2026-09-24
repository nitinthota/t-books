import { all, exec, withTransaction } from "./db";
import { invokeCommand, isTauriRuntime } from "./platform";
import type { VendorAccount, VendorRef } from "./types";

type SqlRow = Record<string, string | number | null>;

function text(row: SqlRow, key: string): string {
  return String(row[key] ?? "").trim();
}

function ensureAccountsTable(): void {
  exec(`CREATE TABLE IF NOT EXISTS vendor_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vendor TEXT NOT NULL COLLATE NOCASE,
    label TEXT,
    bank TEXT,
    account_number TEXT,
    ifsc TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0
  )`);
}

export function listVendorAccountsLocal(vendor: string): VendorAccount[] {
  ensureAccountsTable();
  const name = vendor.trim();
  if (!name) return [];
  const existing = all<SqlRow>(
    `SELECT id, COALESCE(label,'') AS label, COALESCE(bank,'') AS bank,
            COALESCE(account_number,'') AS account_number, COALESCE(ifsc,'') AS ifsc,
            COALESCE(is_primary,0) AS is_primary
     FROM vendor_accounts WHERE vendor = ? COLLATE NOCASE ORDER BY is_primary DESC, id`,
    [name],
  );
  if (existing.length) {
    return existing.map((row) => ({
      id: Number(row.id ?? 0),
      label: text(row, "label"),
      bank: text(row, "bank"),
      accountNumber: text(row, "account_number"),
      ifsc: text(row, "ifsc"),
      isPrimary: Number(row.is_primary ?? 0) !== 0,
    }));
  }
  const master = all<SqlRow>(
    `SELECT COALESCE(bank,'') AS bank, COALESCE(account_number,'') AS account_number, COALESCE(ifsc,'') AS ifsc
     FROM vendors WHERE vendor = ? COLLATE NOCASE`,
    [name],
  )[0];
  if (!master) return [];
  const bank = text(master, "bank");
  const accountNumber = text(master, "account_number");
  const ifsc = text(master, "ifsc");
  if (!bank && !accountNumber && !ifsc) return [];
  return [{ label: "Primary", bank, accountNumber, ifsc, isPrimary: true }];
}

export function saveVendorLocal(payload: { vendor: string; gst: string; accounts: VendorAccount[] }): VendorRef {
  const name = payload.vendor.trim();
  if (!name) throw new Error("Vendor name is required.");
  ensureAccountsTable();
  const accounts = payload.accounts.filter(
    (a) => a.bank.trim() || a.accountNumber.trim() || a.ifsc.trim() || a.label.trim(),
  );
  const list = accounts.length ? accounts : [{ label: "Primary", bank: "", accountNumber: "", ifsc: "", isPrimary: true }];
  if (!list.some((a) => a.isPrimary)) list[0]!.isPrimary = true;
  const primary = list.find((a) => a.isPrimary) ?? list[0]!;
  withTransaction(() => {
    exec(
      `INSERT INTO vendors (vendor, gst, bank, account_number, ifsc, updated_at)
       VALUES (?, ?, ?, ?, ?, datetime('now'))
       ON CONFLICT(vendor) DO UPDATE SET
         gst = excluded.gst,
         bank = excluded.bank,
         account_number = excluded.account_number,
         ifsc = excluded.ifsc,
         updated_at = excluded.updated_at`,
      [name, payload.gst.trim(), primary.bank.trim(), primary.accountNumber.trim(), primary.ifsc.trim()],
    );
    exec("DELETE FROM vendor_accounts WHERE vendor = ? COLLATE NOCASE", [name]);
    list.forEach((acc, i) => {
      const primaryFlag = acc.isPrimary || (i === 0 && !list.some((a) => a.isPrimary)) ? 1 : 0;
      const label = acc.label.trim() || (primaryFlag ? "Primary" : `Account ${i + 1}`);
      exec(
        `INSERT INTO vendor_accounts (vendor, label, bank, account_number, ifsc, is_primary)
         VALUES (?, ?, ?, ?, ?, ?)`,
        [name, label, acc.bank.trim(), acc.accountNumber.trim(), acc.ifsc.trim(), primaryFlag],
      );
    });
  });
  return {
    vendor: name,
    gst: payload.gst.trim(),
    bank: primary.bank,
    accountNumber: primary.accountNumber,
    ifsc: primary.ifsc,
    accounts: listVendorAccountsLocal(name),
  };
}

export async function listVendorAccounts(vendor: string): Promise<VendorAccount[]> {
  if (isTauriRuntime()) {
    return invokeCommand<VendorAccount[]>("list_vendor_accounts", { vendor });
  }
  return listVendorAccountsLocal(vendor);
}

export async function saveVendor(payload: {
  vendor: string;
  gst: string;
  accounts: VendorAccount[];
}): Promise<VendorRef> {
  if (isTauriRuntime()) {
    return invokeCommand<VendorRef>("save_vendor", { payload });
  }
  return saveVendorLocal(payload);
}
