import {
  CREDENTIALS_MISSING_ERROR,
  OFFLINE_SUBMIT_ERROR,
  OFFLINE_VOUCHER_REFRESH_ERROR,
  VOUCHER_RAW_TAB,
} from "./constants";
import { all, exec, getMeta, withTransaction } from "./db";
import { isBrowserOnline } from "./online";
import { summarizeVoucher, voucherFingerprint, type VoucherInput } from "./business_rules";
import {
  invokeCommand,
  isTauriRuntime,
} from "./platform";
import type { PaymentView, RefreshOutcome, SubmitOutcome, VoucherListRow, VoucherSummary, VoucherView } from "./types";
import {
  parseVoucherValues,
  type ParsedVoucher,
  type ParseReport,
} from "./voucher-parse";

export { parseVoucherValues, type ParsedVoucher, type ParseReport };

function asInput(row: ParsedVoucher): VoucherInput {
  return {
    voucher_number: row.voucher_number,
    tax_invoice: row.tax_invoice,
    vendor: row.vendor,
    bank: row.bank,
    account_number: row.account_number,
    ifsc: row.ifsc,
    gst: row.gst,
    project: row.project,
    comments: row.payments
      .map((p) => p.remarks)
      .filter(Boolean)
      .join(" · "),
    payments: row.payments,
  };
}

export function listDirtyVouchers(): number[] {
  return all<{ voucher_number: number }>(
    "SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1 ORDER BY voucher_number",
  ).map((row) => Number(row.voucher_number));
}

export function dirtyGuard(): number[] | null {
  const dirty = listDirtyVouchers();
  return dirty.length > 0 ? dirty : null;
}

function deleteDirtyInTx(): void {
  exec(
    "DELETE FROM voucher_payments WHERE voucher_number IN (SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1)",
  );
  exec("DELETE FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1");
}

export function discardDirtyVouchers(): void {
  withTransaction(() => {
    deleteDirtyInTx();
  });
}

function emptyWarning(skipped: number): string {
  return skipped > 0
    ? `${VOUCHER_RAW_TAB} had no valid voucher numbers. Local vouchers were kept. ${skipped} row(s) skipped.`
    : `${VOUCHER_RAW_TAB} is empty. Local vouchers were kept.`;
}

function applyParsed(parsed: ParseReport, discardDirty: boolean): RefreshOutcome {
  let outcome: RefreshOutcome | null = null;
  withTransaction(() => {
    if (discardDirty) deleteDirtyInTx();

    if (parsed.vouchers.length === 0) {
      exec(`INSERT OR REPLACE INTO app_meta (key, value) VALUES ('voucher_last_synced', datetime('now'))`);
      outcome = {
        kind: "ok",
        imported: 0,
        skipped: parsed.skipped,
        errors: parsed.errors,
        warning: emptyWarning(parsed.skipped),
      };
      return;
    }

    const dirtySet = new Set(
      all<{ voucher_number: number }>(
        "SELECT voucher_number FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1",
      ).map((row) => Number(row.voucher_number)),
    );

    let imported = 0;
    for (const row of parsed.vouchers) {
      if (dirtySet.has(row.voucher_number)) continue;
      const computed = summarizeVoucher(asInput(row));
      const description = row.payments
        .map((p) => p.description)
        .filter(Boolean)
        .join(" · ");
      const comments = row.payments
        .map((p) => p.remarks)
        .filter(Boolean)
        .join(" · ");
      const hash = voucherFingerprint(row);
      exec(
        `INSERT INTO vouchers (
          voucher_number, voucher_date, tax_invoice, vendor, bank, account_number, ifsc, gst, project,
          comments, description, dirty, is_dirty, source, source_hash, fingerprint, total_value, total_paid, remaining, status, tax_flag,
          created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 'sheet', ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        ON CONFLICT(voucher_number) DO UPDATE SET
          voucher_date = excluded.voucher_date,
          tax_invoice = excluded.tax_invoice,
          vendor = excluded.vendor,
          bank = excluded.bank,
          account_number = excluded.account_number,
          ifsc = excluded.ifsc,
          gst = excluded.gst,
          project = excluded.project,
          comments = excluded.comments,
          description = excluded.description,
          dirty = 0,
          is_dirty = 0,
          source = 'sheet',
          source_hash = excluded.source_hash,
          fingerprint = excluded.fingerprint,
          total_value = excluded.total_value,
          total_paid = excluded.total_paid,
          remaining = excluded.remaining,
          status = excluded.status,
          tax_flag = excluded.tax_flag,
          updated_at = excluded.updated_at
        WHERE COALESCE(vouchers.is_dirty, vouchers.dirty, 0) = 0`,
        [
          row.voucher_number,
          row.voucher_date,
          row.tax_invoice,
          row.vendor,
          row.bank,
          row.account_number,
          row.ifsc,
          row.gst,
          row.project,
          comments,
          description,
          hash,
          hash,
          computed.total_value,
          computed.total_paid,
          computed.remaining,
          computed.status,
          computed.tax_flag,
        ],
      );
      exec("DELETE FROM voucher_payments WHERE voucher_number = ?", [row.voucher_number]);
      for (const p of row.payments) {
        exec(
          `INSERT INTO voucher_payments (
            voucher_number, slot, pi_no, pi_date, pi_value_rupees, payment_percent, description,
            tds_rupees, paid_rupees, payment_details, payment_date, remaining_rupees, remarks
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
          [
            row.voucher_number,
            p.slot,
            p.pi_no,
            p.pi_date,
            p.pi_value_rupees,
            p.payment_percent,
            p.description,
            p.tds_rupees,
            p.paid_rupees,
            p.payment_details,
            p.payment_date,
            p.remaining_rupees,
            p.remarks,
          ],
        );
      }
      if (row.vendor) {
        exec(
          `INSERT INTO vendors (vendor, bank, account_number, ifsc, gst, updated_at)
           VALUES (?, ?, ?, ?, ?, datetime('now'))
           ON CONFLICT(vendor) DO UPDATE SET
             bank = excluded.bank,
             account_number = excluded.account_number,
             ifsc = excluded.ifsc,
             gst = excluded.gst,
             updated_at = excluded.updated_at`,
          [row.vendor, row.bank, row.account_number, row.ifsc, row.gst],
        );
      }
      if (row.project) {
        exec(
          `INSERT INTO projects (project, updated_at)
           VALUES (?, datetime('now'))
           ON CONFLICT(project) DO UPDATE SET updated_at = excluded.updated_at`,
          [row.project],
        );
      }
      imported += 1;
    }

    exec(`INSERT OR REPLACE INTO app_meta (key, value) VALUES ('voucher_last_synced', datetime('now'))`);

    outcome = {
      kind: "ok",
      imported,
      skipped: parsed.skipped,
      errors: parsed.errors,
      warning:
        parsed.skipped > 0
          ? `Data refreshed. ${parsed.skipped} row(s) skipped (invalid voucher number).`
          : null,
    };
  });
  return outcome!;
}

export function applyVoucherRows(parsed: ParseReport): RefreshOutcome {
  return applyParsed(parsed, false);
}

export function applyVoucherRowsDiscardingDirty(parsed: ParseReport): RefreshOutcome {
  return applyParsed(parsed, true);
}

export function getVoucherSummary(): VoucherSummary {
  const count = Number(all<{ c: number }>("SELECT COUNT(*) AS c FROM vouchers")[0]?.c ?? 0);
  const dirty = Number(
    all<{ c: number }>(
      "SELECT COUNT(*) AS c FROM vouchers WHERE COALESCE(is_dirty, dirty, 0) = 1",
    )[0]?.c ?? 0,
  );
  const like = (needle: string) =>
    Number(
      all<{ c: number }>("SELECT COUNT(*) AS c FROM vouchers WHERE lower(COALESCE(status, '')) LIKE ?", [
        `%${needle}%`,
      ])[0]?.c ?? 0,
    );
  return {
    count,
    lastSynced: getMeta("voucher_last_synced"),
    pending: like("pending"),
    partial: like("partial"),
    full: like("full"),
    advance: like("advance"),
    missingTax: like("missing"),
    dirty,
    remainingTotal: Number(
      all<{ s: number }>("SELECT COALESCE(SUM(remaining), 0) AS s FROM vouchers")[0]?.s ?? 0,
    ),
  };
}

export function listVouchers(): VoucherListRow[] {
  return all<Record<string, string | number | null>>(
    `SELECT voucher_number, vendor, project, tax_invoice, COALESCE(total_value, 0) AS total_value,
            COALESCE(total_paid, 0) AS total_paid, COALESCE(remaining, 0) AS remaining,
            COALESCE(status, '') AS status, COALESCE(is_dirty, dirty, 0) AS is_dirty
     FROM vouchers ORDER BY voucher_number DESC`,
  ).map((row) => ({
    voucherNumber: Number(row.voucher_number ?? 0),
    vendor: String(row.vendor ?? ""),
    project: String(row.project ?? ""),
    taxInvoice: String(row.tax_invoice ?? ""),
    totalValue: Number(row.total_value ?? 0),
    totalPaid: Number(row.total_paid ?? 0),
    remaining: Number(row.remaining ?? 0),
    status: String(row.status ?? ""),
    isDirty: Number(row.is_dirty ?? 0) !== 0,
  }));
}

export function getVoucher(voucherNumber: number): VoucherView {
  const row = all<Record<string, string | number | null>>(
    `SELECT voucher_number, tax_invoice, vendor, project, gst, comments,
            COALESCE(total_value, 0) AS total_value, COALESCE(total_paid, 0) AS total_paid,
            COALESCE(remaining, 0) AS remaining, COALESCE(status, '') AS status,
            COALESCE(is_dirty, dirty, 0) AS is_dirty
     FROM vouchers WHERE voucher_number = ?`,
    [voucherNumber],
  )[0];
  if (!row) throw new Error(`Voucher ${voucherNumber} is not on this PC.`);
  const payments: PaymentView[] = all<Record<string, string | number | null>>(
    `SELECT slot, pi_no, pi_date, COALESCE(pi_value_rupees, 0) AS pi_value,
            COALESCE(paid_rupees, 0) AS paid, COALESCE(remaining_rupees, 0) AS remaining,
            payment_date, remarks
     FROM voucher_payments WHERE voucher_number = ? ORDER BY slot LIMIT 5`,
    [voucherNumber],
  )
    .map((p) => ({
      slot: Number(p.slot ?? 0),
      piNo: String(p.pi_no ?? ""),
      piDate: String(p.pi_date ?? ""),
      piValue: Number(p.pi_value ?? 0),
      paid: Number(p.paid ?? 0),
      remaining: Number(p.remaining ?? 0),
      paymentDate: String(p.payment_date ?? ""),
      remarks: String(p.remarks ?? ""),
    }))
    .filter(
      (p) => p.piValue !== 0 || p.paid !== 0 || p.piNo.trim() !== "" || p.paymentDate.trim() !== "",
    );
  return {
    voucherNumber: Number(row.voucher_number ?? 0),
    taxInvoice: String(row.tax_invoice ?? ""),
    vendor: String(row.vendor ?? ""),
    project: String(row.project ?? ""),
    gst: String(row.gst ?? ""),
    comments: String(row.comments ?? ""),
    totalValue: Number(row.total_value ?? 0),
    totalPaid: Number(row.total_paid ?? 0),
    remaining: Number(row.remaining ?? 0),
    status: String(row.status ?? ""),
    isDirty: Number(row.is_dirty ?? 0) !== 0,
    payments,
  };
}

export function refreshVouchers(): RefreshOutcome {
  if (!isBrowserOnline()) throw new Error(OFFLINE_VOUCHER_REFRESH_ERROR);
  const dirty = dirtyGuard();
  if (dirty) return { kind: "dirty", voucherNumbers: dirty };
  throw new Error(CREDENTIALS_MISSING_ERROR);
}

export function forceRefreshVouchers(): RefreshOutcome {
  if (!isBrowserOnline()) throw new Error(OFFLINE_VOUCHER_REFRESH_ERROR);
  throw new Error(CREDENTIALS_MISSING_ERROR);
}

export function markDirty(voucherNumber: number): void {
  exec(
    "UPDATE vouchers SET is_dirty = 1, dirty = 1, updated_at = datetime('now') WHERE voucher_number = ?",
    [voucherNumber],
  );
}

export async function loadVoucherList(): Promise<VoucherListRow[]> {
  if (isTauriRuntime()) return invokeCommand<VoucherListRow[]>("list_vouchers");
  return listVouchers();
}

export async function loadVoucher(voucherNumber: number): Promise<VoucherView> {
  if (isTauriRuntime()) {
    return invokeCommand<VoucherView>("get_voucher", { voucherNumber });
  }
  return getVoucher(voucherNumber);
}

export async function submitVoucher(voucherNumber: number): Promise<SubmitOutcome> {
  if (isTauriRuntime()) {
    return invokeCommand<SubmitOutcome>("submit_voucher", { voucherNumber });
  }
  if (!isBrowserOnline()) throw new Error(OFFLINE_SUBMIT_ERROR);
  throw new Error(CREDENTIALS_MISSING_ERROR);
}

export async function reloadVoucher(voucherNumber: number): Promise<VoucherView> {
  if (isTauriRuntime()) {
    return invokeCommand<VoucherView>("reload_voucher", { voucherNumber });
  }
  if (!isBrowserOnline()) throw new Error(OFFLINE_SUBMIT_ERROR);
  throw new Error(CREDENTIALS_MISSING_ERROR);
}
