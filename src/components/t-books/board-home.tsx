import { memo, useEffect, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { runBoardSearch, type BoardAsk, type BoardHit } from "@/lib/t-books/board-search";
import { listPurchasePo, listSalesPo } from "@/lib/t-books/office";
import { invokeCommand, isTauriRuntime } from "@/lib/t-books/platform";
import { useBooks } from "@/lib/t-books/store";
import type { DirtyKey, NavId, PurchasePo, SalesPo } from "@/lib/t-books/types";
import { discardAllDirty, discardDirtyKey, listDirtyKeys } from "@/lib/t-books/vouchers";

function navForDirty(kind: string): NavId {
  if (kind === "purchase" || kind === "payment") return "purchase";
  if (kind === "sales_po") return "sales";
  if (kind === "salary") return "hr";
  if (kind === "inventory") return "inventory";
  if (kind === "logistics") return "logistics";
  if (kind === "document") return "documents";
  return "vouchers";
}

export const BoardHome = memo(function BoardHome({
  onOpen,
}: {
  onOpen: (nav: NavId, key?: string) => void;
}) {
  const summary = useBooks((s) => s.voucherSummary);
  const lastError = useBooks((s) => s.lastError);
  const [keys, setKeys] = useState<DirtyKey[]>([]);
  const [bills, setBills] = useState<PurchasePo[]>([]);
  const [orders, setOrders] = useState<SalesPo[]>([]);
  const [askText, setAskText] = useState("");
  const [ask, setAsk] = useState<BoardAsk | null>(null);
  const [hits, setHits] = useState<BoardHit[] | null>(null);
  const [looking, setLooking] = useState(false);

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

  async function onAsk(event?: { preventDefault(): void }) {
    event?.preventDefault();
    const raw = askText.trim();
    if (!raw) {
      setAsk(null);
      setHits(null);
      return;
    }
    setLooking(true);
    try {
      const out = await runBoardSearch(raw, bills, orders);
      setAsk(out.ask);
      setHits(out.hits);
    } finally {
      setLooking(false);
    }
  }

  async function reloadKeys() {
    try {
      const list = isTauriRuntime()
        ? await invokeCommand<DirtyKey[]>("get_dirty_keys")
        : listDirtyKeys();
      setKeys(list);
    } catch {
      /* keep last */
    }
  }

  async function dropOne(row: DirtyKey) {
    if (!window.confirm(`Remove ${row.kind} ${row.key} from this PC?`)) return;
    try {
      if (isTauriRuntime()) {
        await invokeCommand("discard_dirty_key", { kind: row.kind, key: row.key });
      } else {
        discardDirtyKey(row.kind, row.key);
      }
      await reloadKeys();
    } catch {
      /* leave the list */
    }
  }

  async function dropAll() {
    if (!window.confirm("Remove every not-posted draft from this PC?")) return;
    try {
      if (isTauriRuntime()) {
        await invokeCommand("discard_all_dirty");
      } else {
        discardAllDirty();
      }
      await reloadKeys();
    } catch {
      /* leave the list */
    }
  }

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
      <form onSubmit={(e) => void onAsk(e)} className="mb-6">
        <label className="block text-xs uppercase tracking-wide text-ink-subtle" htmlFor="board-ask">
          Ask the books
        </label>
        <div className="mt-1.5 flex gap-2">
          <input
            id="board-ask"
            value={askText}
            onChange={(e) => setAskText(e.target.value)}
            placeholder="How much did we pay Star Engineering?"
            className="h-11 flex-1 rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
          />
          <button
            type="submit"
            className="pressable h-11 rounded-md bg-navy px-4 text-sm text-white"
            disabled={looking}
          >
            {looking ? "Looking…" : "Look"}
          </button>
        </div>
        {ask?.changed ? (
          <p className="mt-2 text-sm text-ink-muted">
            Reading as: <span className="text-ink">{ask.cleaned}</span>
          </p>
        ) : (
          <p className="mt-2 text-xs text-ink-subtle">
            Type a sentence. Spelling is tidied on this PC, then we look in the books. Nothing goes to Google.
          </p>
        )}
        {hits && hits.length === 0 ? (
          <p className="mt-3 text-sm text-ink-muted">Nothing on this PC matches that.</p>
        ) : null}
        {hits && hits.length > 0 ? (
          <ul className="mt-3 max-h-56 overflow-auto rounded-lg bg-paper-raised ring-1 ring-line">
            {hits.map((hit) => (
              <li key={`${hit.nav}-${hit.key}`} className="border-b border-line last:border-0">
                <button
                  type="button"
                  className="pressable w-full px-3 py-2 text-left"
                  onClick={() => onOpen(hit.nav, hit.key)}
                >
                  <span className="block text-sm text-ink">{hit.title}</span>
                  {hit.subtitle ? <span className="block text-xs text-ink-muted">{hit.subtitle}</span> : null}
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </form>
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
          onClick={() => onOpen(keys[0] ? navForDirty(keys[0].kind) : "vouchers")}
        />
      </div>
      {keys.length > 0 ? (
        <div className="mt-6 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
          <div className="flex items-center justify-between gap-3">
            <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Not posted</p>
            <button
              type="button"
              className="pressable rounded-md px-3 py-1 text-xs text-ink-muted ring-1 ring-line"
              onClick={() => void dropAll()}
            >
              Remove all
            </button>
          </div>
          <p className="mt-1 text-xs text-ink-subtle">On this PC only. Google is not changed.</p>
          <ul className="mt-2 max-h-48 overflow-auto text-sm">
            {keys.map((k) => (
              <li key={`${k.kind}-${k.key}`} className="flex items-center gap-2">
                <button
                  type="button"
                  className="pressable min-w-0 flex-1 py-1 text-left"
                  onClick={() => onOpen(navForDirty(k.kind), k.key)}
                >
                  {k.kind} {k.key}
                </button>
                <button
                  type="button"
                  className="pressable shrink-0 rounded-md px-2 py-1 text-xs text-ink-muted ring-1 ring-line"
                  onClick={() => void dropOne(k)}
                >
                  Remove
                </button>
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
