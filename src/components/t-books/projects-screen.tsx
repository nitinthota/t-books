import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { listInventory, listLogistics, listProjects, listPurchasePo, listSalesPo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { InventoryRow, LogisticsRow, NavId, PurchasePo, SalesPo, VoucherListRow } from "@/lib/t-books/types";
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
  tripCount: number;
  stockCount: number;
  vendors: string[];
  bills: PurchasePo[];
  orders: SalesPo[];
};

type JobFilter = "all" | "due" | "paid" | "sales" | "purchase" | "trips" | "empty";

function jobWord(row: ProjectRollup): string {
  if (row.due > 0.5) return "Still to pay";
  if (row.billed > 0.5) return "Settled";
  if (row.salesCount) return "Customer PO";
  if (row.purchaseCount || row.vouchers || row.tripCount) return "On the job";
  return "Name only";
}

function matchesFilter(row: ProjectRollup, filter: JobFilter): boolean {
  const hasDocs = row.vouchers > 0 || row.purchaseCount > 0 || row.salesCount > 0 || row.tripCount > 0;
  switch (filter) {
    case "due":
      return row.due > 0.5;
    case "paid":
      return row.billed > 0.5 && row.due <= 0.5;
    case "sales":
      return row.salesCount > 0;
    case "purchase":
      return row.purchaseCount > 0;
    case "trips":
      return row.tripCount > 0;
    case "empty":
      return !hasDocs;
    default:
      return true;
  }
}

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
  const [trips, setTrips] = useState<LogisticsRow[]>([]);
  const [stock, setStock] = useState<InventoryRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<JobFilter>("all");
  const [openName, setOpenName] = useState<string | null>(focusId?.trim() || null);

  const reload = useCallback(async () => {
    try {
      const [projectNames, voucherRows, bills, pos, tripRows, stockRows] = await Promise.all([
        listProjects(),
        loadVoucherList(),
        listPurchasePo(),
        listSalesPo(),
        listLogistics().catch(() => [] as LogisticsRow[]),
        listInventory().catch(() => [] as InventoryRow[]),
      ]);
      setNames(projectNames);
      setVouchers(voucherRows);
      setPurchases(bills);
      setSales(pos);
      setTrips(tripRows);
      setStock(stockRows);
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
    for (const row of trips) if (row.project.trim()) extra.add(row.project.trim());
    for (const row of stock) if (row.project.trim()) extra.add(row.project.trim());
    const all = [...new Set([...(names ?? []), ...extra])].sort((a, b) => a.localeCompare(b));
    return all.map((project) => {
      const key = project.toLowerCase();
      const v = vouchers.filter((row) => row.project.trim().toLowerCase() === key);
      const p = purchases.filter((row) => row.project.trim().toLowerCase() === key);
      const s = sales.filter((row) => row.project.trim().toLowerCase() === key);
      const t = trips.filter((row) => row.project.trim().toLowerCase() === key);
      const i = stock.filter((row) => row.project.trim().toLowerCase() === key);
      const billed = v.reduce((sum, row) => sum + (row.totalValue || 0), 0);
      const paid = v.reduce((sum, row) => sum + (row.totalPaid || 0), 0);
      const purPaid = p.reduce((sum, row) => sum + (row.paidRupees || 0), 0);
      const customerPo = s.reduce((sum, row) => sum + (row.totalValue || 0), 0);
      const vendors = [
        ...new Set(
          [...v.map((row) => row.vendor), ...p.map((row) => row.vendor)].map((name) => name.trim()).filter(Boolean),
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
        tripCount: t.length,
        stockCount: i.length,
        vendors,
        bills: p,
        orders: s,
      };
    });
  }, [names, vouchers, purchases, sales, trips, stock]);

  const counts = useMemo(() => {
    const tally = { all: rows.length, due: 0, paid: 0, sales: 0, purchase: 0, trips: 0, empty: 0 };
    for (const row of rows) {
      if (matchesFilter(row, "due")) tally.due += 1;
      if (matchesFilter(row, "paid")) tally.paid += 1;
      if (matchesFilter(row, "sales")) tally.sales += 1;
      if (matchesFilter(row, "purchase")) tally.purchase += 1;
      if (matchesFilter(row, "trips")) tally.trips += 1;
      if (matchesFilter(row, "empty")) tally.empty += 1;
    }
    return tally;
  }, [rows]);

  const focused = focusId?.trim() ?? "";
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return rows.filter((row) => {
      if (!matchesFilter(row, filter)) return false;
      if (q && !row.project.toLowerCase().includes(q) && !row.vendors.some((v) => v.toLowerCase().includes(q))) return false;
      return true;
    });
  }, [rows, query, filter]);

  const open = rows.find((row) => row.project === openName) ?? null;

  if (open) {
    return (
      <ModuleFrame title={open.project} hint="Job file. Counts open that book for this job only." onBack={() => setOpenName(null)} error={error}>
        <p className="text-sm text-ink-muted">{jobWord(open)}</p>
        <dl className="mt-4 grid gap-3 sm:grid-cols-4 text-sm">
          <Money label="Billed" value={formatRupees(open.billed)} />
          <Money label="Paid" value={formatRupees(open.paid)} />
          <Money label="Still to pay" value={formatRupees(open.due)} />
          <Money label="Customer PO" value={formatRupees(open.customerPo)} />
        </dl>
        <div className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          <CountCard label="Vouchers" value={String(open.vouchers)} onClick={() => onOpen?.("vouchers", open.project)} />
          <CountCard label="Purchase bills" value={String(open.purchaseCount)} onClick={() => onOpen?.("purchase", open.project)} />
          <CountCard label="Sales orders" value={String(open.salesCount)} onClick={() => onOpen?.("sales", open.project)} />
          <CountCard label="Trips" value={String(open.tripCount)} onClick={() => onOpen?.("logistics", open.project)} />
          <CountCard label="Stock lines" value={String(open.stockCount)} onClick={() => onOpen?.("inventory", open.project)} />
          <CountCard label="Vendors" value={String(open.vendors.length)} />
        </div>
        <Section title="Purchase bills on this job">
          {open.bills.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No purchase bill on this job yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {open.bills.map((bill) => (
                <li key={bill.id}>
                  <button type="button" className="pressable flex w-full items-center justify-between px-3 py-2.5 text-left text-sm" onClick={() => onOpen?.("purchase", bill.poNumber || open.project)}>
                    <span>
                      {bill.poNumber || "PUR"}
                      <span className="ml-2 text-ink-muted">{bill.vendor}</span>
                    </span>
                    <span className="tabular-nums">{formatRupees(bill.totalValue)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Section>
        <Section title="Sales orders on this job">
          {open.orders.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No sales order on this job yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {open.orders.map((order) => (
                <li key={order.id}>
                  <button type="button" className="pressable flex w-full items-center justify-between px-3 py-2.5 text-left text-sm" onClick={() => onOpen?.("sales", order.poNumber || open.project)}>
                    <span>
                      {order.poNumber || "SAL"}
                      <span className="ml-2 text-ink-muted">{order.client}</span>
                    </span>
                    <span className="tabular-nums">{formatRupees(order.totalValue)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Section>
        <Section title="Vendors on this job">
          {open.vendors.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No vendor on this job yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {open.vendors.map((vendor) => (
                <li key={vendor}>
                  <button type="button" className="pressable w-full px-3 py-2.5 text-left text-sm" onClick={() => onOpen?.("vendors", vendor)}>
                    {vendor}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Section>
      </ModuleFrame>
    );
  }

  const chips: Array<{ id: JobFilter; label: string; count: number }> = [
    { id: "all", label: "All jobs", count: counts.all },
    { id: "due", label: "Still to pay", count: counts.due },
    { id: "paid", label: "Settled", count: counts.paid },
    { id: "sales", label: "Customer PO", count: counts.sales },
    { id: "purchase", label: "Purchase bills", count: counts.purchase },
    { id: "trips", label: "Trips", count: counts.trips },
    { id: "empty", label: "Name only", count: counts.empty },
  ];

  return (
    <ModuleFrame title="Projects" hint="Job master. Open a job to see money, bills, trips, and stock." onBack={onBack} error={error}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Job or vendor"
        aria-label="Search jobs"
        className="mb-3 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="mb-4 flex flex-wrap gap-2" role="tablist" aria-label="Job filters">
        {chips.map((chip) => {
          const on = filter === chip.id;
          return (
            <button
              key={chip.id}
              type="button"
              role="tab"
              aria-selected={on}
              onClick={() => setFilter(chip.id)}
              className={
                on
                  ? "pressable rounded-md bg-navy px-3 py-1.5 text-xs text-white"
                  : "pressable rounded-md px-3 py-1.5 text-xs text-ink-muted ring-1 ring-line"
              }
            >
              {chip.label} {chip.count}
            </button>
          );
        })}
      </div>
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.project}
          empty={
            <p className="text-sm text-ink-muted">
              {names === null ? "Opening jobs\u2026" : query.trim() || filter !== "all" ? "No job matches." : "No jobs yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Job</TableHeadCell>
              <TableHeadCell>State</TableHeadCell>
              <TableHeadCell className="text-right">Bills</TableHeadCell>
              <TableHeadCell className="text-right">Still to pay</TableHeadCell>
              <TableHeadCell className="text-right">Customer PO</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr
              className={row.project === focused ? "table-row cursor-pointer bg-navy-soft" : "table-row cursor-pointer"}
              onClick={() => setOpenName(row.project)}
            >
              <TableCell>
                {row.project}
                <span className="mt-0.5 block text-xs text-ink-muted">
                  {row.vouchers} vouchers \u00b7 {row.vendors.length} vendors
                </span>
              </TableCell>
              <TableCell className="text-ink-muted">{jobWord(row)}</TableCell>
              <TableCell className="text-right tabular-nums">{row.purchaseCount}</TableCell>
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
  if (!onClick) return <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">{inner}</div>;
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

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mt-6">
      <p className="text-xs uppercase tracking-wide text-ink-subtle">{title}</p>
      {children}
    </div>
  );
}
