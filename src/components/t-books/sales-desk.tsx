import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees } from "@/lib/t-books/business_rules";
import { getSalesPo, listPurchasePo, listSalesPo, saveSalesPo, submitOffice } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePo, SalesPo } from "@/lib/t-books/types";
import { DeskChips } from "./desk-chips";
import { DeskSearch } from "./desk-search";
import { ModuleFrame } from "./module-frame";
import {
  draftsFromItems,
  parseItemDrafts,
  PoItemsEditor,
  type ItemDraft,
} from "./po-items-editor";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

type SalesFilter = "all" | "due" | "unpaid" | "paid" | "contract" | "project" | "dirty";
type ObjectFacet = "overview" | "lines" | "related";

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
  const [filter, setFilter] = useState<SalesFilter>("all");
  const [facet, setFacet] = useState<ObjectFacet>("overview");

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusId]);

  async function openCard(id: number) {
    setBusy(true);
    try {
      const po = await getSalesPo(id);
      setCard(po);
      setLines(draftsFromItems(po.items ?? []));
      setFacet("overview");
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function writeLines() {
    if (!card) return null;
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
    return saved;
  }

  async function saveLines() {
    if (!card) return;
    setBusy(true);
    try {
      await writeLines();
      await reload();
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function postOrder() {
    if (!card) return;
    setBusy(true);
    try {
      const saved = await writeLines();
      if (!saved) return;
      const key = `${saved.poNumber.trim()}@${saved.project.trim()}`;
      const out = await submitOffice("sales_po", key);
      if (out.kind === "conflict") setError(out.message);
      else {
        await reload();
        setError(null);
      }
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
      if (filter === "unpaid" && !((row.receivedRupees ?? 0) <= 0.5 && (row.totalValue || 0) > 0.5))
        return false;
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
      unpaid: list.filter(
        (row) => (row.receivedRupees ?? 0) <= 0.5 && (row.totalValue || 0) > 0.5,
      ).length,
      paid: list.filter((row) => (row.totalValue || 0) > 0.5 && dueOf(row) <= 0.5).length,
      contract: list.filter((row) => String(row.kind || "contract") === "contract").length,
      project: list.filter((row) => String(row.kind || "") === "project").length,
      dirty: list.filter((row) => Boolean(row.isDirty)).length,
    };
  }, [rows]);

  if (card) {
    const received = card.receivedRupees ?? 0;
    const balance = card.totalValue - received;
    const jobKey = card.project.trim().toLowerCase();
    const relatedBills = jobKey
      ? bills.filter((row) => row.project.trim().toLowerCase() === jobKey)
      : [];
    const lineCount = lines.filter((row) => row.description.trim()).length || lines.length;

    return (
      <ModuleFrame
        title={card.poNumber || "Sales order"}
        onBack={() => {
          setCard(null);
          setFacet("overview");
        }}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void saveLines()} disabled={busy}>
              {busy ? "Saving\u2026" : "Save draft"}
            </Button>
            <Button onClick={() => void postOrder()} disabled={busy}>
              {busy ? "Posting\u2026" : "Post"}
            </Button>
            <Button variant="ghost" onClick={() => onEdit(card.id)}>
              Full edit
            </Button>
          </div>
        }
      >
        <div className="grid gap-3 sm:grid-cols-4">
          <Kpi label="Value" value={formatRupees(card.totalValue)} />
          <Kpi label="Received" value={formatRupees(received)} />
          <Kpi label="Balance" value={formatRupees(balance)} accent />
          <Kpi label="Lines" value={String(lineCount)} />
        </div>

        <div className="mt-5 flex flex-wrap gap-2" role="tablist" aria-label="Sales sections">
          {(
            [
              ["overview", "Overview"],
              ["lines", `Lines ${lineCount}`],
              ["related", `Related ${relatedBills.length}`],
            ] as const
          ).map(([id, label]) => {
            const on = facet === id;
            return (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={on}
                onClick={() => setFacet(id)}
                className={
                  on
                    ? "pressable h-9 rounded-full bg-navy px-3 text-xs text-white"
                    : "pressable h-9 rounded-full bg-paper-raised px-3 text-xs text-ink-muted ring-1 ring-line"
                }
              >
                {label}
              </button>
            );
          })}
        </div>

        {facet === "overview" ? (
          <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2">
            <Fact
              label="Project"
              value={card.project}
              onOpen={card.project && onOpenProject ? () => onOpenProject(card.project) : undefined}
            />
            <Fact label="Client" value={card.client} />
            <Fact label="Kind" value={card.kind === "project" ? "Project" : "Contract"} />
            <Fact label="GST" value={card.gst} />
          </dl>
        ) : null}

        {facet === "lines" ? (
          <div className="mt-6">
            <PoItemsEditor items={lines} onChange={setLines} />
          </div>
        ) : null}

        {facet === "related" ? (
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
        ) : null}
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Sales"
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
      <DeskSearch
        value={query}
        onChange={setQuery}
        placeholder="Order, job, client"
        label="Search sales"
      />
      <DeskChips
        label="Sales filters"
        value={filter}
        onChange={setFilter}
        items={[
          { id: "all", label: "All orders", count: counts.all },
          { id: "due", label: "Balance due", count: counts.due },
          { id: "unpaid", label: "No receipt", count: counts.unpaid },
          { id: "paid", label: "Settled", count: counts.paid },
          { id: "contract", label: "Contract PO", count: counts.contract },
          { id: "project", label: "Project PO", count: counts.project },
          { id: "dirty", label: "Not posted", count: counts.dirty },
        ]}
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null
                ? "Opening sales\u2026"
                : job || filter !== "all"
                  ? "No order matches."
                  : "No sales orders yet."}
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
                <TableCell className="font-medium">
                  {row.poNumber}
                  {row.isDirty ? <span className="ml-2 text-xs text-gold">Draft</span> : null}
                </TableCell>
                <TableCell className="text-ink-muted">{row.project || "\u2014"}</TableCell>
                <TableCell className="text-ink-muted">{row.client || "\u2014"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
                <TableCell className="text-right tabular-nums text-navy">
                  {formatRupees(row.totalValue - received)}
                </TableCell>
              </tr>
            );
          }}
        />
      </div>
    </ModuleFrame>
  );
}

function Kpi({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className={`money-figure mt-1 text-2xl ${accent ? "text-navy" : ""}`}>{value}</p>
    </div>
  );
}

function Fact({ label, value, onOpen }: { label: string; value: string; onOpen?: () => void }) {
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
