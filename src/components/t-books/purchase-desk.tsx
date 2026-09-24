import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees, paymentOrdinal } from "@/lib/t-books/business_rules";
import { getPurchasePo, listPurchasePayments, listPurchasePo, savePurchasePo } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePayment, PurchasePo, VoucherListRow } from "@/lib/t-books/types";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";
import { draftsFromItems, parseItemDrafts, PoItemsEditor, type ItemDraft } from "./po-items-editor";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function PurchaseDesk({
  onBack,
  focusId,
  onOpenProject,
  onOpenVendor,
  onEdit,
  onNew,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpenProject?: (project: string) => void;
  onOpenVendor?: (vendor: string) => void;
  onEdit: (id: number) => void;
  onNew: () => void;
}) {
  const [rows, setRows] = useState<PurchasePo[] | null>(null);
  const [pays, setPays] = useState<PurchasePayment[]>([]);
  const [vouchers, setVouchers] = useState<VoucherListRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [card, setCard] = useState<PurchasePo | null>(null);
  const [lines, setLines] = useState<ItemDraft[]>([]);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "due" | "unpaid" | "paid" | "missing_tax" | "contract" | "simple">("all");

  const reload = useCallback(async () => {
    try {
      const [list, paymentRows, voucherRows] = await Promise.all([
        listPurchasePo(),
        listPurchasePayments(),
        loadVoucherList(),
      ]);
      setRows(list);
      setPays(paymentRows);
      setVouchers(voucherRows);
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

  function dueOf(row: PurchasePo): number {
    return (row.totalValue || 0) - (row.paidRupees ?? 0);
  }

  function billState(row: PurchasePo): "unpaid" | "due" | "paid" {
    const due = dueOf(row);
    const paid = row.paidRupees ?? 0;
    if (paid <= 0.5 && due > 0.5) return "unpaid";
    if (due > 0.5) return "due";
    return "paid";
  }

  const job = query.trim().toLowerCase();
  const visible = useMemo(() => {
    const list = rows ?? [];
    return list.filter((row) => {
      if (filter === "unpaid" && billState(row) !== "unpaid") return false;
      if (filter === "due" && !(dueOf(row) > 0.5)) return false;
      if (filter === "paid" && !(dueOf(row) <= 0.5 && (row.totalValue || 0) > 0.5)) return false;
      if (filter === "missing_tax" && (row.taxInvoiceNo || "").trim()) return false;
      if (filter === "contract" && row.type !== "contract") return false;
      if (filter === "simple" && row.type !== "simple") return false;
      if (!job) return true;
      return (
        row.project.toLowerCase().includes(job) ||
        row.vendor.toLowerCase().includes(job) ||
        row.poNumber.toLowerCase().includes(job)
      );
    });
  }, [rows, job, filter]);

  const counts = useMemo(() => {
    const list = rows ?? [];
    return {
      all: list.length,
      due: list.filter((row) => dueOf(row) > 0.5).length,
      unpaid: list.filter((row) => billState(row) === "unpaid").length,
      paid: list.filter((row) => dueOf(row) <= 0.5 && (row.totalValue || 0) > 0.5).length,
      missing_tax: list.filter((row) => !(row.taxInvoiceNo || "").trim()).length,
      contract: list.filter((row) => row.type === "contract").length,
      simple: list.filter((row) => row.type === "simple").length,
    };
  }, [rows]);

  if (card) {
    const billPays = pays
      .filter((p) => p.poNumber.toLowerCase() === card.poNumber.toLowerCase())
      .slice()
      .sort((a, b) => a.payNumber.localeCompare(b.payNumber));
    const jobKey = card.project.trim().toLowerCase();
    const relatedVouchers = jobKey
      ? vouchers.filter((row) => row.project.trim().toLowerCase() === jobKey)
      : [];
    const due = card.totalValue - (card.paidRupees ?? 0);
    return (
      <ModuleFrame
        title={card.poNumber || "Purchase bill"}
        hint="This bill only. Lines and payments belong here."
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
          <Count label="Payments" value={String(billPays.length)} />
          <Count label="Still to pay" value={formatRupees(due)} />
        </div>
        <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2">
          <Fact
            label="Project"
            value={card.project}
            onOpen={card.project && onOpenProject ? () => onOpenProject(card.project) : undefined}
          />
          <Fact
            label="Vendor"
            value={card.vendor}
            onOpen={card.vendor && onOpenVendor ? () => onOpenVendor(card.vendor) : undefined}
          />
          <Fact label="Type" value={card.type === "simple" ? "Project expenses" : "Techsol purchase"} />
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
              {billPays.map((p, i) => (
                <li key={p.payNumber} className="flex justify-between px-3 py-2 text-sm">
                  <span>
                    {p.payNumber}
                    <span className="ml-2 text-ink-muted">{paymentOrdinal(i + 1)}</span>
                  </span>
                  <span className="tabular-nums">{formatRupees(p.amountRupees)}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="mt-6">
          <p className="text-xs uppercase tracking-wide text-ink-subtle">Vouchers on this job</p>
          {relatedVouchers.length === 0 ? (
            <p className="mt-2 text-sm text-ink-muted">No voucher on this job yet.</p>
          ) : (
            <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
              {relatedVouchers.map((row) => (
                <li key={row.voucherNumber} className="flex justify-between px-3 py-2 text-sm">
                  <span>{row.voucherNumber}</span>
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
    <ModuleFrame title="Purchase" hint="Open a bill to see every line. Add or delete rows there." onBack={onBack} error={error} actions={<Button onClick={onNew}>New bill</Button>}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Bill, job, vendor"
        aria-label="Search purchase"
        className="mb-3 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="mb-4 flex flex-wrap gap-2" role="tablist" aria-label="Purchase filters">
        {(
          [
            ["all", "All bills", counts.all],
            ["due", "Still to pay", counts.due],
            ["unpaid", "No payment", counts.unpaid],
            ["paid", "Settled", counts.paid],
            ["missing_tax", "No tax invoice", counts.missing_tax],
            ["contract", "Techsol Purchase", counts.contract],
            ["simple", "Project Expenses", counts.simple],
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
              {rows === null ? "Opening purchase\u2026" : job || filter !== "all" ? "No bill matches." : "No purchase bills yet."}
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
              <TableCell className="text-ink-muted">{row.project || "\u2014"}</TableCell>
              <TableCell className="text-ink-muted">{row.vendor || "\u2014"}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
              <TableCell className="text-ink-muted">{row.payStatus || "Unpaid"}</TableCell>
            </tr>
          )}
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
