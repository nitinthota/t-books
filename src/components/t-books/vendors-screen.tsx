import { useCallback, useEffect, useMemo, useState } from "react";
import { formatBankLabel, formatRupees } from "@/lib/t-books/business_rules";
import { listVendors } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { VendorRef, VoucherListRow } from "@/lib/t-books/types";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

type VendorRow = VendorRef & {
  bankLine: string;
  billed: number;
  due: number;
  vouchers: number;
};

export default function VendorsScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const [rows, setRows] = useState<VendorRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      const [vendors, list] = await Promise.all([listVendors(), loadVoucherList()]);
      const byVendor = new Map<string, VoucherListRow[]>();
      for (const row of list) {
        const key = row.vendor.trim().toLowerCase();
        if (!key) continue;
        const cur = byVendor.get(key) ?? [];
        cur.push(row);
        byVendor.set(key, cur);
      }
      const enriched: VendorRow[] = vendors.map((vendor) => {
        const linked = byVendor.get(vendor.vendor.trim().toLowerCase()) ?? [];
        return {
          ...vendor,
          bankLine: formatBankLabel({
            bank_name: vendor.bank,
            account_no: vendor.accountNumber,
            ifsc: vendor.ifsc,
          }),
          vouchers: linked.length,
          billed: linked.reduce((sum, row) => sum + (row.totalValue || 0), 0),
          due: linked.reduce((sum, row) => sum + (row.remaining || 0), 0),
        };
      });
      setRows(enriched);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      setRows((prev) => prev ?? []);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const focused = focusId?.trim() ?? "";
  const visible = useMemo(() => rows ?? [], [rows]);

  return (
    <ModuleFrame
      title="Vendors"
      hint="From this PC. Bank line is formatBankLabel from listVendors (bank / accountNumber / ifsc). Refresh does not delete names."
      onBack={onBack}
      error={error}
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.vendor}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening vendors on this PC…" : "No vendors on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell>GST</TableHeadCell>
              <TableHeadCell>Bank</TableHeadCell>
              <TableHeadCell className="text-right">Due</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className={row.vendor === focused ? "table-row bg-navy-soft" : "table-row"}>
              <TableCell>
                {row.vendor}
                <span className="ml-2 text-xs text-ink-subtle">{row.vouchers} voucher(s)</span>
              </TableCell>
              <TableCell className="text-ink-muted">{row.gst || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.bankLine || "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.due)}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}
