import { useCallback, useEffect, useMemo, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { listProjects, listPurchasePo, listSalesPo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePo, SalesPo, VoucherListRow } from "@/lib/t-books/types";
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
};

export default function ProjectsScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const [names, setNames] = useState<string[] | null>(null);
  const [vouchers, setVouchers] = useState<VoucherListRow[]>([]);
  const [purchases, setPurchases] = useState<PurchasePo[]>([]);
  const [sales, setSales] = useState<SalesPo[]>([]);
  const [error, setError] = useState<string | null>(null);

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

  const rows: ProjectRollup[] = useMemo(() => {
    const extra = new Set<string>();
    for (const row of vouchers) if (row.project.trim()) extra.add(row.project.trim());
    for (const row of purchases) if (row.project.trim()) extra.add(row.project.trim());
    for (const row of sales) if (row.project.trim()) extra.add(row.project.trim());
    const all = [...new Set([...(names ?? []), ...extra])].sort((a, b) => a.localeCompare(b));
    return all.map((project) => {
      const key = project.toLowerCase();
      const v = vouchers.filter((row) => row.project.trim().toLowerCase() === key);
      const billed = v.reduce((sum, row) => sum + (row.totalValue || 0), 0);
      const paid = v.reduce((sum, row) => sum + (row.totalPaid || 0), 0);
      const purPaid = purchases
        .filter((row) => row.project.trim().toLowerCase() === key)
        .reduce((sum, row) => sum + (row.paidRupees || 0), 0);
      const customerPo = sales
        .filter((row) => row.project.trim().toLowerCase() === key)
        .reduce((sum, row) => sum + (row.totalValue || 0), 0);
      return {
        project,
        vouchers: v.length,
        billed,
        paid: paid + purPaid,
        due: billed - paid,
        customerPo,
      };
    });
  }, [names, vouchers, purchases, sales]);

  const focused = focusId?.trim() ?? "";

  return (
    <ModuleFrame
      title="Projects"
      hint="Rollup from this PC: voucher billed / paid / due plus customer PO. Refresh does not invent jobs."
      onBack={onBack}
      error={error}
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows}
          rowKey={(row) => row.project}
          empty={
            <p className="text-sm text-ink-muted">
              {names === null ? "Opening projects on this PC…" : "No projects on this PC yet."}
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
            <tr className={row.project === focused ? "table-row bg-navy-soft" : "table-row"}>
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
