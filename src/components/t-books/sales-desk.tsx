import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees } from "@/lib/t-books/business_rules";
import { getSalesPo, listPurchasePo, listSalesPo, saveSalesPo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePo, SalesPo } from "@/lib/t-books/types";
import { ModuleFrame } from "./module-frame";
import {
  draftsFromItems,
  parseItemDrafts,
  PoItemsEditor,
  type ItemDraft,
} from "./po-items-editor";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function SalesDesk({
  onBack,
  focusId,
  onOpenProject,
  onEdit,
  onNew,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpenProject?: (project: string) => void;
  onEdit: (id: number) => void;
  onNew: (kind: "contract" | "project") => void;
}) {
  const [rows, setRows] = useState<SalesPo[] | null>(null);
  const [bills, setBills] = useState<PurchasePo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [card, setCard] = useState<SalesPo | null>(null);
  const [lines, setLines] = useState<ItemDraft[]>([]);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "due" | "unpaid" | "paid" | "contract" | "project" | "dirty">("all");

  const reload = useCallback(async () => {
    try {
      const [list, purchaseRows] = await Promise.all([listSalesPo(), listPurchasePo()]);
      setRows(list);
      setBills(purchaseRows);
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
    if (!focusId) return;
    const id = Number(focusId);
    if (Number.isFinite(id) && id > 0) void openCard(id);
    else setQuery(focusId);
  }, [focusId]);

  async function openCard(id: number) {
    setBusy(true);
    try {
      const po = await getSalesPo(id);
      setCard(po);
      setLines(draftsFromItems(po.items ?? []));
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function saveLines() {
    if (!card) return;
    setBusy(true);
    try {
      const items = parseItemDrafts(lines);
      const saved = await saveSalesPo({
        id: card.id,
        project: card.project,
        poNumber: card.poNumber,
        client: card.client,
        gst: card.gst,
        items,
        kind: card.kind === "project" ? "project" : "contract",
        receivedRupees: card.receivedRupees ?? 0,
        paymentTermValue: card.paymentTermValue ?? 0,
        paymentTermUnit: card.paymentTermUnit === "months" ? "months" : "days",
      });
      setCard(saved);
      setLines(draftsFromItems(saved.items ?? []));
      await reload();
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  function dueOf(row: SalesPo): number {
    return (row.totalValue || 0) - (row.receivedRupees ?? 0);
  }

  const job = query.trim().toLowerCase();
  const visible = useMemo(() => {
    const list = rows ?? [];
    return list.filter((row) => {
      const kind = String(row.kind || "contract");
      if (filter === "due" && !(dueOf(row) > 0.5)) return false;
      if (filter === "unpaid" && !((row.receivedRupees ?? 0) <= 0.5 && (row.totalValue || 0) > 0.5)) return false;
      if (filter === "paid" && !((row.totalValue || 0) > 0.5 && dueOf(row) <= 0.5)) return false;
      if (filter === "contract" && kind !== "contract") return false;
      if (filter === "project" && kind !== "project") return false;
      if (filter === "dirty" && !row.isDirty) return false;
      if (!job) return true;
      return (
        row.project.toLowerCase().includes(job) ||
        row.client.toLowerCase().includes(job) ||
        row.poNumber.toLowerCase().includes(job)
      );
    });
  }, [rows, job, filter]);

  const counts = useMemo(() => {
    const list = rows ?? [];
    return {
      all: list.length,
      due: list.filter((row) => dueOf(row) > 0.5).length,
      unpaid: list.filter((row) => (row.receivedRupees ?? 0) <= 0.5 && (row.totalValue || 0) > 0.5).length,
      paid: list.filter((row) => (row.totalValue || 0) > 0.5 && dueOf(row) <= 0.5).length,
      contract: list.filter((row) => String(row.kind || "contract") === "contract").length,
      project: list.filter((row) => String(row.kind || "") === "project").length,
      dirty: list.filter((row) => Boolean(row.isDirty)).length,
    };
  }, [rows]);

  if (card) {
    const received = card.receivedRupees ?? 0;
    const jobKey = card.project.trim().toLowerCase();
    const relatedBills = jobKey
      ? bills.filter((row) => row.project.trim().toLowerCase() === jobKey)
      : [];
    return (
      <ModuleFrame
        title={card.poNumber || "Sales order"}
        hint="This order only. Lines belong here."
        onBack={() => setCard(null)}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void saveLines()} disabled={busy}>
              {busy ? "Saving\u2026" : "Save lines"}
            </Button>
            <Button onClick={() => onEdit(card.id)}>Full edit</Button>
          </div>
        }
      >
        <div className="grid gap-3 sm:grid-cols-3">
          <Count label="Lines" value={String(lines.filter((row) => row.description.trim()).length || lines.length)} />
          <Count label="Received" value={formatRupees(received)} />
          <Count label="Balance" value={formatRupees(card.totalValue - received)} />
        </div>
        <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2">
          <Item
            label="Project"
            value={card.project}
            onOpen={card.project && onOpenProject ? () => onOpenProject(card.project) : undefined}
          />
          <Item label="Client" value={card.client} />
          <Item label="Kind" value={card.kind === "project" ? "Project" : "Contract"} />
          <Item label="GST" value={card.gst} />
          <Item label="Value" value={formatRupees(card.totalValue)} />
        </dl>
        <div className="mt-6">
          <PoItemsEditor items={lines} onChange={setLines} />
        </div>
        <div className="mt-6">
          <p className="text-xs uppercase tracking-wide text-ink-subtle">Purchase bills on this job</p>
          {relatedBills.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No purchase bill on this job yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {relatedBills.map((row) => (
                <li key={row.id} className="flex justify-between px-3 py-2 text-sm">
                  <span>{row.poNumber}</span>
                  <span className="text-ink-muted">{row.vendor}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Sales"
      hint="Open an order to see every line. Add or delete rows there."
      onBack={onBack}
      error={error}
      actions={
        <div className="flex gap-2">
          <Button variant="secondary" onClick={() => onNew("project")}>
            New project PO
          </Button>
          <Button onClick={() => onNew("contract")}>New contract PO</Button>
        </div>
      }
    >
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Order, job, client"
        aria-label="Search sales"
        className="mb-3 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="mb-4 flex flex-wrap gap-2" role="tablist" aria-label="Sales filters">
        {(
          [
            ["all", "All orders", counts.all],
            ["due", "Balance due", counts.due],
            ["unpaid", "No receipt", counts.unpaid],
            ["paid", "Settled", counts.paid],
            ["contract", "Contract PO", counts.contract],
            ["project", "Project PO", counts.project],
            ["dirty", "Not posted", counts.dirty],
          ] as const
        ).map(([id, label, count]) => {
          const on = filter === id;
          return (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={on}
              onClick={() => setFilter(id)}
              className={
                on
                  ? "pressable rounded-md bg-navy px-3 py-1.5 text-xs text-white"
                  : "pressable rounded-md px-3 py-1.5 text-xs text-ink-muted ring-1 ring-line"
              }
            >
              {label} {count}
            </button>
          );
        })}
      </div>
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening sales\u2026" : job || filter !== "all" ? "No order matches." : "No sales orders yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>PO</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell>Client</TableHeadCell>
              <TableHeadCell className="text-right">Value</TableHeadCell>
              <TableHeadCell className="text-right">Balance</TableHeadCell>
            </tr>
          }
          renderRow={(row) => {
            const received = row.receivedRupees ?? 0;
            return (
              <tr className="table-row cursor-pointer" onClick={() => void openCard(row.id)}>
                <TableCell>
                  {row.poNumber}
                  {row.isDirty ? <span className="ml-2 text-xs text-gold">unsynced</span> : null}
                </TableCell>
                <TableCell className="text-ink-muted">{row.project || "\u2014"}</TableCell>
                <TableCell className="text-ink-muted">{row.client || "\u2014"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue - received)}</TableCell>
              </tr>
            );
          }}
        />
      </div>
    </ModuleFrame>
  );
}

function Count({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className="money-figure mt-1 text-2xl">{value}</p>
    </div>
  );
}

function Item({ label, value, onOpen }: { label: string; value: string; onOpen?: () => void }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-ink-subtle">{label}</dt>
      <dd className="mt-1">
        {onOpen && value ? (
          <button type="button" className="text-navy hover:underline" onClick={onOpen}>
            {value}
          </button>
        ) : (
          value || "\u2014"
        )}
      </dd>
    </div>
  );
}
