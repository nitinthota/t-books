import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees } from "@/lib/t-books/business_rules";
import { getPurchasePo, listPurchasePayments, listPurchasePo, savePurchasePo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePayment, PurchasePo } from "@/lib/t-books/types";
import { ModuleFrame } from "./module-frame";
import { draftsFromItems, parseItemDrafts, PoItemsEditor, type ItemDraft } from "./po-items-editor";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function PurchaseDesk({
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
  onNew: () => void;
}) {
  const [rows, setRows] = useState<PurchasePo[] | null>(null);
  const [pays, setPays] = useState<PurchasePayment[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [card, setCard] = useState<PurchasePo | null>(null);
  const [lines, setLines] = useState<ItemDraft[]>([]);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");

  const reload = useCallback(async () => {
    try {
      const [list, paymentRows] = await Promise.all([listPurchasePo(), listPurchasePayments()]);
      setRows(list);
      setPays(paymentRows);
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
      const po = await getPurchasePo(id);
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
      const saved = await savePurchasePo({
        id: card.id,
        project: card.project,
        vendor: card.vendor,
        poNumber: card.poNumber,
        type: card.type === "simple" ? "simple" : "contract",
        totalValue: card.totalValue,
        items: parseItemDrafts(lines),
        goodsReceived: card.goodsReceived,
        taxInvoiceNo: card.taxInvoiceNo,
        taxInvoiceDate: card.taxInvoiceDate,
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

  const job = query.trim().toLowerCase();
  const visible = useMemo(() => {
    const list = rows ?? [];
    if (!job) return list;
    return list.filter(
      (row) =>
        row.project.toLowerCase().includes(job) ||
        row.vendor.toLowerCase().includes(job) ||
        row.poNumber.toLowerCase().includes(job),
    );
  }, [rows, job]);

  if (card) {
    const billPays = pays.filter((p) => p.poNumber.toLowerCase() === card.poNumber.toLowerCase());
    return (
      <ModuleFrame
        title={card.poNumber || "Purchase bill"}
        hint="Lines belong on this bill. Add, change or delete a row, then Save."
        onBack={() => setCard(null)}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void saveLines()} disabled={busy}>
              {busy ? "Saving…" : "Save lines"}
            </Button>
            <Button onClick={() => onEdit(card.id)}>Full edit</Button>
          </div>
        }
      >
        <dl className="grid gap-3 text-sm sm:grid-cols-2">
          <Fact
            label="Project"
            value={card.project}
            onOpen={card.project && onOpenProject ? () => onOpenProject(card.project) : undefined}
          />
          <Fact label="Vendor" value={card.vendor} />
          <Fact label="Type" value={card.type === "simple" ? "Simple" : "Contract"} />
          <Fact label="Status" value={card.payStatus || "Unpaid"} />
          <Fact label="Value" value={formatRupees(card.totalValue)} />
          <Fact label="Paid" value={formatRupees(card.paidRupees ?? 0)} />
        </dl>
        <div className="mt-6">
          <PoItemsEditor items={lines} onChange={setLines} />
        </div>
        <div className="mt-6">
          <p className="text-xs uppercase tracking-wide text-ink-subtle">Payments</p>
          {billPays.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No payment on this bill yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {billPays.map((p) => (
                <li key={p.payNumber} className="flex justify-between px-3 py-2 text-sm">
                  <span>{p.payNumber}</span>
                  <span className="tabular-nums">{formatRupees(p.amountRupees)}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame title="Purchase" hint="Open a bill to see every line. Add or delete rows there." onBack={onBack} error={error} actions={<Button onClick={onNew}>New bill</Button>}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Bill, job, vendor"
        aria-label="Search purchase"
        className="mb-4 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening purchase…" : job ? "No bill matches." : "No purchase bills yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>PUR</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell className="text-right">Value</TableHeadCell>
              <TableHeadCell>Status</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row cursor-pointer" onClick={() => void openCard(row.id)}>
              <TableCell className="font-mono">{row.poNumber}</TableCell>
              <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.vendor || "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
              <TableCell className="text-ink-muted">{row.payStatus || "Unpaid"}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
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
          value || "—"
        )}
      </dd>
    </div>
  );
}
