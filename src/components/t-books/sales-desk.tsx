import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees } from "@/lib/t-books/business_rules";
import { listSalesPo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { SalesPo } from "@/lib/t-books/types";
import { ModuleFrame } from "./module-frame";
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
  const [error, setError] = useState<string | null>(null);
  const [card, setCard] = useState<SalesPo | null>(null);
  const [query, setQuery] = useState("");

  const reload = useCallback(async () => {
    try {
      setRows(await listSalesPo());
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
    if (Number.isFinite(id) && id > 0) onEdit(id);
    else setQuery(focusId);
  }, [focusId, onEdit]);

  const job = query.trim().toLowerCase();
  const visible = useMemo(() => {
    const list = rows ?? [];
    if (!job) return list;
    return list.filter(
      (row) =>
        row.project.toLowerCase().includes(job) ||
        row.client.toLowerCase().includes(job) ||
        row.poNumber.toLowerCase().includes(job),
    );
  }, [rows, job]);

  if (card) {
    const received = card.receivedRupees ?? 0;
    return (
      <ModuleFrame
        title={card.poNumber || "Sales order"}
        hint="This order only."
        onBack={() => setCard(null)}
        error={error}
        actions={<Button onClick={() => onEdit(card.id)}>Edit</Button>}
      >
        <dl className="grid gap-3 text-sm sm:grid-cols-2">
          <Item
            label="Project"
            value={card.project}
            onOpen={card.project && onOpenProject ? () => onOpenProject(card.project) : undefined}
          />
          <Item label="Client" value={card.client} />
          <Item label="Kind" value={card.kind === "project" ? "Project" : "Contract"} />
          <Item label="GST" value={card.gst} />
          <Item label="Value" value={formatRupees(card.totalValue)} />
          <Item label="Received" value={formatRupees(received)} />
          <Item label="Balance" value={formatRupees(card.totalValue - received)} />
          <Item label="Lines" value={String(card.items?.length ?? 0)} />
        </dl>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Sales"
      hint="Open an order for the full picture. Click the job to open that job."
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
        className="mb-4 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening sales…" : job ? "No order matches." : "No sales orders yet."}
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
              <tr className="table-row cursor-pointer" onClick={() => setCard(row)}>
                <TableCell>
                  {row.poNumber}
                  {row.isDirty ? <span className="ml-2 text-xs text-gold">unsynced</span> : null}
                </TableCell>
                <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
                <TableCell className="text-ink-muted">{row.client || "—"}</TableCell>
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
          value || "—"
        )}
      </dd>
    </div>
  );
}
