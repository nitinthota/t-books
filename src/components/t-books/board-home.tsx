import { memo, useEffect, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { listPurchasePo, listSalesPo } from "@/lib/t-books/office";
import { invokeCommand, isTauriRuntime } from "@/lib/t-books/platform";
import { useBooks } from "@/lib/t-books/store";
import type { DirtyKey, NavId, PurchasePo, SalesPo } from "@/lib/t-books/types";
import { listDirtyKeys } from "@/lib/t-books/vouchers";

export const BoardHome = memo(function BoardHome({
  onOpen,
}: {
  onOpen: (nav: NavId) => void;
}) {
  const summary = useBooks((s) => s.voucherSummary);
  const lastError = useBooks((s) => s.lastError);
  const [keys, setKeys] = useState<DirtyKey[]>([]);
  const [bills, setBills] = useState<PurchasePo[]>([]);
  const [orders, setOrders] = useState<SalesPo[]>([]);

  useEffect(() => {
    void (async () => {
      try {
        const list = isTauriRuntime()
          ? await invokeCommand<DirtyKey[]>("get_dirty_keys")
          : listDirtyKeys();
        setKeys(list);
      } catch {
        /* keep last */
      }
      try {
        const [po, sales] = await Promise.all([listPurchasePo(), listSalesPo()]);
        setBills(po);
        setOrders(sales);
      } catch {
        /* board still shows voucher figures */
      }
    })();
  }, [summary, lastError]);

  const voucherCount = summary?.count ?? 0;
  const voucherDue = summary?.remainingTotal ?? 0;
  const unsynced = keys.length || summary?.dirty || 0;
  const purchaseDue = bills.reduce((sum, row) => {
    const left = (row.totalValue || 0) - (row.paidRupees ?? 0);
    return sum + (left > 0 ? left : 0);
  }, 0);
  const salesBalance = orders.reduce((sum, row) => {
    const left = (row.totalValue || 0) - (row.receivedRupees ?? 0);
    return sum + (left > 0 ? left : 0);
  }, 0);

  return (
    <div className="mt-8">
      {lastError ? (
        <p className="mb-4 text-sm text-gold">Refresh failed. Previous figures are kept.</p>
      ) : null}
      <p className="text-xs text-ink-subtle">
        {summary?.lastSynced ? `Last refreshed ${summary.lastSynced}` : "Not refreshed yet."}
      </p>
      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <Tile
          label="Vouchers"
          value={String(voucherCount)}
          hint={`${formatRupees(voucherDue)} still to pay`}
          onClick={() => onOpen("vouchers")}
        />
        <Tile
          label="Purchase bills"
          value={String(bills.length)}
          hint={`${formatRupees(purchaseDue)} unpaid`}
          onClick={() => onOpen("purchase")}
        />
        <Tile
          label="Sales orders"
          value={String(orders.length)}
          hint={`${formatRupees(salesBalance)} balance`}
          onClick={() => onOpen("sales")}
        />
        <Tile label="Jobs" value="Open" hint="Billed, paid, still due" onClick={() => onOpen("projects")} />
        <Tile label="Vendors" value="Open" hint="Party card and jobs" onClick={() => onOpen("vendors")} />
        <Tile
          label="Not posted"
          value={String(unsynced)}
          hint={unsynced ? "Parked here" : "Nothing waiting"}
          gold={unsynced > 0}
          onClick={() => onOpen("vouchers")}
        />
      </div>
      {keys.length > 0 ? (
        <div className="mt-6 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Not posted</p>
          <ul className="mt-2 max-h-40 overflow-auto text-sm">
            {keys.map((k) => (
              <li key={`${k.kind}-${k.key}`} className="py-1">
                {k.kind} {k.key}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
});

function Tile({
  label,
  value,
  hint,
  onClick,
  gold,
}: {
  label: string;
  value: string;
  hint: string;
  onClick: () => void;
  gold?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="board-card pressable rounded-lg bg-paper-raised p-5 text-left ring-1 ring-line"
    >
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className={gold ? "money-figure mt-1 text-3xl text-gold" : "money-figure mt-1 text-3xl text-navy"}>{value}</p>
      <p className="mt-2 text-sm text-ink-muted">{hint}</p>
    </button>
  );
}
