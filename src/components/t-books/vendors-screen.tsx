import { useCallback, useEffect, useMemo, useState } from "react";
import { formatBankLabel, formatRupees } from "@/lib/t-books/business_rules";
import { listPurchasePo, listSalesPo, listVendors } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { NavId, VendorRef } from "@/lib/t-books/types";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

type VendorRow = VendorRef & {
  bankLine: string;
  billed: number;
  paid: number;
  due: number;
  vouchers: number;
  purchaseCount: number;
  salesCount: number;
  jobs: string[];
};

export default function VendorsScreen({
  onBack,
  focusId,
  onOpen,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpen?: (nav: NavId, key: string) => void;
}) {
  const [rows, setRows] = useState<VendorRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [openName, setOpenName] = useState<string | null>(focusId?.trim() || null);

  const reload = useCallback(async () => {
    try {
      const [vendors, list, bills, sales] = await Promise.all([
        listVendors(),
        loadVoucherList(),
        listPurchasePo(),
        listSalesPo(),
      ]);
      const extra = new Map<string, string>();
      for (const row of list) if (row.vendor.trim()) extra.set(row.vendor.trim().toLowerCase(), row.vendor.trim());
      for (const row of bills) if (row.vendor.trim()) extra.set(row.vendor.trim().toLowerCase(), row.vendor.trim());
      const names = new Map<string, VendorRef>();
      for (const vendor of vendors) names.set(vendor.vendor.trim().toLowerCase(), vendor);
      for (const [key, label] of extra) {
        if (!names.has(key)) names.set(key, { vendor: label, gst: "" });
      }
      const enriched: VendorRow[] = [...names.values()].map((vendor) => {
        const key = vendor.vendor.trim().toLowerCase();
        const v = list.filter((row) => row.vendor.trim().toLowerCase() === key);
        const p = bills.filter((row) => row.vendor.trim().toLowerCase() === key);
        const s = sales.filter((row) => row.client.trim().toLowerCase() === key);
        const jobs = [...new Set([...v, ...p].map((row) => row.project.trim()).filter(Boolean))].sort((a, b) =>
          a.localeCompare(b),
        );
        return {
          ...vendor,
          bankLine: formatBankLabel({
            bank_name: vendor.bank,
            account_no: vendor.accountNumber,
            ifsc: vendor.ifsc,
          }),
          vouchers: v.length,
          purchaseCount: p.length,
          salesCount: s.length,
          billed: v.reduce((sum, row) => sum + (row.totalValue || 0), 0),
          paid: v.reduce((sum, row) => sum + (row.totalPaid || 0), 0),
          due: v.reduce((sum, row) => sum + (row.remaining || 0), 0),
          jobs,
        };
      });
      enriched.sort((a, b) => a.vendor.localeCompare(b.vendor));
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

  useEffect(() => {
    if (focusId?.trim()) setOpenName(focusId.trim());
  }, [focusId]);

  const visible = useMemo(() => {
    const list = rows ?? [];
    const q = query.trim().toLowerCase();
    if (!q) return list;
    return list.filter((row) =>
      [row.vendor, row.gst, row.bankLine, ...row.jobs].some((v) => v.toLowerCase().includes(q)),
    );
  }, [rows, query]);

  const open = (rows ?? []).find((row) => row.vendor === openName) ?? null;

  if (open) {
    return (
      <ModuleFrame title={open.vendor} hint="This vendor only." onBack={() => setOpenName(null)} error={error}>
        <p className="text-sm text-ink-muted">
          {open.gst ? `GST ${open.gst}` : "No GST"}
          {open.bankLine ? ` · ${open.bankLine}` : ""}
        </p>
        <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          <CountCard label="Vouchers" value={String(open.vouchers)} onClick={() => onOpen?.("vouchers", open.vendor)} />
          <CountCard
            label="Purchase bills"
            value={String(open.purchaseCount)}
            onClick={() => onOpen?.("purchase", open.vendor)}
          />
          <CountCard label="Sales as client" value={String(open.salesCount)} onClick={() => onOpen?.("sales", open.vendor)} />
        </div>
        <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-3">
          <Money label="Billed" value={formatRupees(open.billed)} />
          <Money label="Paid" value={formatRupees(open.paid)} />
          <Money label="Still to pay" value={formatRupees(open.due)} />
        </dl>
        <div className="mt-6">
          <p className="text-xs uppercase tracking-wide text-ink-subtle">Jobs</p>
          {open.jobs.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No job on this vendor yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {open.jobs.map((job) => (
                <li key={job}>
                  <button
                    type="button"
                    className="pressable w-full px-3 py-2.5 text-left text-sm text-navy"
                    onClick={() => onOpen?.("projects", job)}
                  >
                    {job}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame title="Vendors" hint="Open a vendor for counts. Click a count to open those documents." onBack={onBack} error={error}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Vendor, GST, job"
        aria-label="Search vendors"
        className="mb-4 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.vendor}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening vendors…" : query.trim() ? "No vendor matches." : "No vendors yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell>GST</TableHeadCell>
              <TableHeadCell>Bank</TableHeadCell>
              <TableHeadCell className="text-right">Vouchers</TableHeadCell>
              <TableHeadCell className="text-right">Still to pay</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row cursor-pointer" onClick={() => setOpenName(row.vendor)}>
              <TableCell>{row.vendor}</TableCell>
              <TableCell className="text-ink-muted">{row.gst || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.bankLine || "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{row.vouchers}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.due)}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}

function CountCard({
  label,
  value,
  onClick,
}: {
  label: string;
  value: string;
  onClick?: () => void;
}) {
  return (
    <button type="button" className="pressable rounded-lg bg-paper-raised p-4 text-left ring-1 ring-line" onClick={onClick}>
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className="money-figure mt-1 text-3xl">{value}</p>
    </button>
  );
}

function Money({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-ink-subtle">{label}</dt>
      <dd className="mt-1 text-ink">{value}</dd>
    </div>
  );
}
