import { memo, useEffect, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { runBoardSearch, type BoardAsk, type BoardHit } from "@/lib/t-books/board-search";
import { listProjects, listPurchasePo, listSalesPo, listVendors, submitOffice } from "@/lib/t-books/office";
import { invokeCommand, isTauriRuntime } from "@/lib/t-books/platform";
import { useBooks } from "@/lib/t-books/store";
import type { DirtyKey, NavId, PurchasePo, SalesPo } from "@/lib/t-books/types";
import { discardAllDirty, discardDirtyKey } from "@/lib/t-books/dirty-discard";
import { listDirtyKeys, submitVoucher } from "@/lib/t-books/vouchers";

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
  const [jobCount, setJobCount] = useState(0);
  const [vendorCount, setVendorCount] = useState(0);
  const [askText, setAskText] = useState("");
  const [ask, setAsk] = useState<BoardAsk | null>(null);
  const [hits, setHits] = useState<BoardHit[] | null>(null);
  const [looking, setLooking] = useState(false);
  const [posting, setPosting] = useState(false);
  const [postNote, setPostNote] = useState<string | null>(null);

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
        const [po, sales, projects, vendors] = await Promise.all([
          listPurchasePo(),
          listSalesPo(),
          listProjects(),
          listVendors(),
        ]);
        setBills(po);
        setOrders(sales);
        const jobs = new Set<string>();
        for (const name of projects) if (name.trim()) jobs.add(name.trim().toLowerCase());
        for (const row of po) if (row.project.trim()) jobs.add(row.project.trim().toLowerCase());
        for (const row of sales) if (row.project.trim()) jobs.add(row.project.trim().toLowerCase());
        setJobCount(jobs.size);
        const parties = new Set<string>();
        for (const row of vendors) if (row.vendor.trim()) parties.add(row.vendor.trim().toLowerCase());
        for (const row of po) if (row.vendor.trim()) parties.add(row.vendor.trim().toLowerCase());
        for (const row of sales) if (row.client.trim()) parties.add(row.client.trim().toLowerCase());
        setVendorCount(parties.size);
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
    if (!window.confirm(`Remove unsaved ${row.kind} ${row.key}?`)) return;
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

  async function postAll() {
    if (!keys.length) return;
    if (!window.confirm(`Post ${keys.length} draft(s) one by one?`)) return;
    setPosting(true);
    setPostNote(null);
    let done = 0;
    try {
      for (const row of keys) {
        if (row.kind === "voucher") {
          const n = Number(row.key);
          const out = await submitVoucher(n);
          if (out.kind === "conflict") {
            setPostNote(`${row.key} was just updated by another user. Reload and submit again.`);
            await reloadKeys();
            return;
          }
        } else {
          const out = await submitOffice(row.kind, row.key);
          if (out.kind === "conflict") {
            setPostNote(out.message);
            await reloadKeys();
            return;
          }
        }
        done += 1;
      }
      setPostNote(done ? `${done} posted.` : "Nothing to post.");
      await reloadKeys();
    } catch (err) {
      setPostNote(err instanceof Error ? err.message : String(err));
    } finally {
      setPosting(false);
    }
  }

  async function dropAll() {
    if (!window.confirm("Remove every unsaved draft?")) return;
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
        <p className="mb-4 text-sm text-gold">Refresh failed. The last figures are kept.</p>
      ) : null}
      <form onSubmit={(e) => void onAsk(e)} className="mb-6">
        <label className="block text-xs uppercase tracking-wide text-ink-subtle" htmlFor="board-ask">
          Search
        </label>
        <div className="mt-1.5 flex gap-2">
          <input
            id="board-ask"
            value={askText}
            onChange={(e) => setAskText(e.target.value)}
            placeholder="Search"
            className="h-11 flex-1 rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
          />
          <button type="submit" className="pressable h-11 rounded-md bg-navy px-4 text-sm text-white" disabled={looking}>
            {looking ? "Looking\u2026" : "Look"}
          </button>
        </div>
      </form>
      <p className="text-xs text-ink-subtle">{summary?.lastSynced ? `Last refreshed ${summary.lastSynced}` : "Not refreshed yet."}</p>
      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <Tile label="Vouchers" value={String(voucherCount)} hint={`${formatRupees(voucherDue)} still to pay`} onClick={() => onOpen("vouchers")} />
        <Tile label="Purchase bills" value={String(bills.length)} hint={`${formatRupees(purchaseDue)} unpaid`} onClick={() => onOpen("purchase")} />
        <Tile label="Sales orders" value={String(orders.length)} hint={`${formatRupees(salesBalance)} balance`} onClick={() => onOpen("sales")} />
        <Tile label="Jobs" value={String(jobCount)} onClick={() => onOpen("projects")} />
        <Tile label="Vendors" value={String(vendorCount)} onClick={() => onOpen("vendors")} />
        <Tile label="Not posted" value={String(unsynced)} hint={unsynced ? "Unsaved drafts" : "All posted"} gold={unsynced > 0} onClick={() => onOpen(keys[0] ? navForDirty(keys[0].kind) : "vouchers")} />
      </div>
      {postNote ? <p className="mt-4 text-sm text-ink-muted">{postNote}</p> : null}
      {keys.length > 0 ? (
        <div className="mt-6 rounded-lg bg-paper-raised p-4 ring-1 ring-line">
          <div className="flex items-center justify-between gap-3">
            <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Not posted</p>
            <div className="flex gap-2">
              <button type="button" className="pressable rounded-md bg-navy px-3 py-1 text-xs text-white" disabled={posting} onClick={() => void postAll()}>{posting ? "Posting\u2026" : "Post all"}</button>
              <button type="button" className="pressable rounded-md px-3 py-1 text-xs text-ink-muted ring-1 ring-line" onClick={() => void dropAll()}>Remove all</button>
            </div>
          </div>
          <ul className="mt-2 max-h-48 overflow-auto text-sm">
            {keys.map((k) => (
              <li key={`${k.kind}-${k.key}`} className="flex items-center gap-2">
                <button type="button" className="pressable min-w-0 flex-1 py-1 text-left" onClick={() => onOpen(navForDirty(k.kind), k.key)}>{k.kind} {k.key}</button>
                <button type="button" className="pressable shrink-0 rounded-md px-2 py-1 text-xs text-ink-muted ring-1 ring-line" onClick={() => void dropOne(k)}>Remove</button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
});

function Tile({
  label, value, hint, onClick, gold,
}: {
  label: string; value: string; hint?: string; onClick: () => void; gold?: boolean;
}) {
  return (
    <button type="button" onClick={onClick} className="board-card pressable rounded-lg bg-paper-raised p-5 text-left ring-1 ring-line">
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className={gold ? "money-figure mt-1 text-3xl text-gold" : "money-figure mt-1 text-3xl text-navy"}>{value}</p>
      {hint ? <p className="mt-2 text-sm text-ink-muted">{hint}</p> : null}
    </button>
  );
}
