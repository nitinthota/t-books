import { useCallback, useEffect, useMemo, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { listProjects, listPurchasePo, listSalesPo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { NavId, PurchasePo, SalesPo, VoucherListRow } from "@/lib/t-books/types";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

type ProjectRollup = {
  project: string;
  vouchers: number;
  billed: number;
  paid: number;
  due: number;
  customerPo: number;
  purchaseCount: number;
  salesCount: number;
  vendors: string[];
};

export default function ProjectsScreen({
  onBack,
  focusId,
  onOpen,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpen?: (nav: NavId, project: string) => void;
}) {
  const [names, setNames] = useState<string[] | null>(null);
  const [vouchers, setVouchers] = useState<VoucherListRow[]>([]);
  const [purchases, setPurchases] = useState<PurchasePo[]>([]);
  const [sales, setSales] = useState<SalesPo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [openName, setOpenName] = useState<string | null>(focusId?.trim() || null);

  const reload = useCallback(async () => {
    try {
      const [projectNames, voucherRows, bills, pos] = await Promise.all([
        listProjects(),
        loadVoucherList(),
        listPurchasePo(),
        listSalesPo(),
      ]);
      setNames(projectNames);
      setVouchers(voucherRows);
      setPurchases(bills);
      setSales(pos);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      setNames((prev) => prev ?? []);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (focusId?.trim()) setOpenName(focusId.trim());
  }, [focusId]);

  const rows: ProjectRollup[] = useMemo(() => {
    const extra = new Set<string>();
    for (const row of vouchers) if (row.project.trim()) extra.add(row.project.trim());
    for (const row of purchases) if (row.project.trim()) extra.add(row.project.trim());
    for (const row of sales) if (row.project.trim()) extra.add(row.project.trim());
    const all = [...new Set([...(names ?? []), ...extra])].sort((a, b) => a.localeCompare(b));
    return all.map((project) => {
      const key = project.toLowerCase();
      const v = vouchers.filter((row) => row.project.trim().toLowerCase() === key);
      const p = purchases.filter((row) => row.project.trim().toLowerCase() === key);
      const s = sales.filter((row) => row.project.trim().toLowerCase() === key);
      const billed = v.reduce((sum, row) => sum + (row.totalValue || 0), 0);
      const paid = v.reduce((sum, row) => sum + (row.totalPaid || 0), 0);
      const purPaid = p.reduce((sum, row) => sum + (row.paidRupees || 0), 0);
      const customerPo = s.reduce((sum, row) => sum + (row.totalValue || 0), 0);
      const vendors = [
        ...new Set(
          [...v.map((row) => row.vendor), ...p.map((row) => row.vendor)]
            .map((name) => name.trim())
            .filter(Boolean),
        ),
      ].sort((a, b) => a.localeCompare(b));
      return {
        project,
        vouchers: v.length,
        billed,
        paid: paid + purPaid,
        due: billed - paid,
        customerPo,
        purchaseCount: p.length,
        salesCount: s.length,
        vendors,
      };
    });
  }, [names, vouchers, purchases, sales]);

  const focused = focusId?.trim() ?? "";
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((row) => row.project.toLowerCase().includes(q));
  }, [rows, query]);

  const open = rows.find((row) => row.project === openName) ?? null;

  if (open) {
    return (
      <ModuleFrame title={open.project} hint="This job only." onBack={() => setOpenName(null)} error={error}>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <CountCard
            label="Vouchers"
            value={String(open.vouchers)}
            onClick={() => onOpen?.("vouchers", open.project)}
          />
          <CountCard
            label="Purchase bills"
            value={String(open.purchaseCount)}
            onClick={() => onOpen?.("purchase", open.project)}
          />
          <CountCard
            label="Sales orders"
            value={String(open.salesCount)}
            onClick={() => onOpen?.("sales", open.project)}
          />
          <CountCard label="Vendors" value={String(open.vendors.length)} />
        </div>
        <dl className="mt-6 grid gap-3 sm:grid-cols-3 text-sm">
          <Money label="Billed" value={formatRupees(open.billed)} />
          <Money label="Paid" value={formatRupees(open.paid)} />
          <Money label="Still to pay" value={formatRupees(open.due)} />
        </dl>
        <div className="mt-6">
          <p className="text-xs uppercase tracking-wide text-ink-subtle">Vendors on this job</p>
          {open.vendors.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No vendor on this job yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg ring-1 ring-line bg-paper-raised">
              {open.vendors.map((vendor) => (
                <li key={vendor}>
                  <button
                    type="button"
                    className="pressable w-full px-3 py-2.5 text-left text-sm"
                    onClick={() => onOpen?.("vendors", vendor)}
                  >
                    {vendor}
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
    <ModuleFrame title="Projects" hint="Open a job for counts. Click a count to open those documents." onBack={onBack} error={error}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Job name"
        aria-label="Search jobs"
        className="mb-4 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.project}
          empty={
            <p className="text-sm text-ink-muted">
              {names === null ? "Opening jobs…" : query.trim() ? "No job matches." : "No jobs yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell className="text-right">Vouchers</TableHeadCell>
              <TableHeadCell className="text-right">Billed</TableHeadCell>
              <TableHeadCell className="text-right">Paid</TableHeadCell>
              <TableHeadCell className="text-right">Due</TableHeadCell>
              <TableHeadCell className="text-right">Customer PO</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr
              className={row.project === focused ? "table-row cursor-pointer bg-navy-soft" : "table-row cursor-pointer"}
              onClick={() => setOpenName(row.project)}
            >
              <TableCell>{row.project}</TableCell>
              <TableCell className="text-right tabular-nums">{row.vouchers}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.billed)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.paid)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.due)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.customerPo)}</TableCell>
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
  const inner = (
    <>
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className="money-figure mt-1 text-3xl">{value}</p>
    </>
  );
  if (!onClick) {
    return <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">{inner}</div>;
  }
  return (
    <button type="button" className="pressable rounded-lg bg-paper-raised p-4 text-left ring-1 ring-line" onClick={onClick}>
      {inner}
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
