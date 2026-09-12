import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatRupees, paymentMatchesFilters, type PaymentFilterKey } from "@/lib/t-books/business_rules";
import {
  deletePurchasePayment,
  deletePurchasePo,
  getPurchasePo,
  listProjects,
  listPurchasePayments,
  listPurchasePo,
  listVendors,
  savePurchasePayment,
  savePurchasePo,
  submitOffice,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePayment, PurchasePo, PurchaseType } from "@/lib/t-books/types";
import { cn } from "@/lib/utils";
import { ModuleFrame, SuggestField } from "./module-frame";
import { draftsFromItems, emptyItem, parseItemDrafts, PoItemsEditor, type ItemDraft } from "./po-items-editor";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";
import { useRegisterUnsaved, useUnsaved } from "./unsaved-guard";

type Draft = {
  id: number | null;
  project: string;
  vendor: string;
  poNumber: string;
  type: PurchaseType;
  totalValue: string;
  goodsReceived: boolean;
  taxInvoiceNo: string;
  taxInvoiceDate: string;
  items: ItemDraft[];
};

const FILTERS: { key: PaymentFilterKey; label: string }[] = [
  { key: "advance", label: "Advance" },
  { key: "partial", label: "Partial" },
  { key: "fully_paid", label: "Fully Paid" },
  { key: "missing_tax", label: "Missing Tax Inv" },
];

function blankDraft(): Draft {
  return {
    id: null,
    project: "",
    vendor: "",
    poNumber: "",
    type: "contract",
    totalValue: "0",
    goodsReceived: false,
    taxInvoiceNo: "",
    taxInvoiceDate: "",
    items: [emptyItem()],
  };
}

function fromPo(po: PurchasePo): Draft {
  return {
    id: po.id,
    project: po.project,
    vendor: po.vendor,
    poNumber: po.poNumber,
    type: po.type,
    totalValue: String(po.totalValue ?? 0),
    goodsReceived: Boolean(po.goodsReceived),
    taxInvoiceNo: po.taxInvoiceNo ?? "",
    taxInvoiceDate: po.taxInvoiceDate ?? "",
    items: draftsFromItems(po.items ?? []),
  };
}

export default function PurchaseScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const { requestLeave } = useUnsaved();
  const [rows, setRows] = useState<PurchasePo[] | null>(null);
  const [payments, setPayments] = useState<PurchasePayment[]>([]);
  const [projects, setProjects] = useState<string[]>([]);
  const [vendors, setVendors] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "edit">("list");
  const [draft, setDraft] = useState<Draft>(blankDraft);
  const [saved, setSaved] = useState<Draft>(blankDraft);
  const [busy, setBusy] = useState(false);
  const [filters, setFilters] = useState<PaymentFilterKey[]>([]);
  const [payAmount, setPayAmount] = useState("");
  const dirty = mode === "edit" && JSON.stringify(draft) !== JSON.stringify(saved);

  const reload = useCallback(async () => {
    try {
      const [list, pays, projectNames, vendorRows] = await Promise.all([
        listPurchasePo(),
        listPurchasePayments(),
        listProjects(),
        listVendors(),
      ]);
      setRows(list);
      setPayments(pays);
      setProjects(projectNames);
      setVendors(vendorRows.map((v) => v.vendor));
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      if (!rows) setRows([]);
    }
  }, [rows]);

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!focusId) return;
    const id = Number(focusId);
    if (Number.isFinite(id) && id > 0) void openEdit(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusId]);

  const writeBill = useCallback(async () => {
    const items = draft.type === "simple" ? [] : parseItemDrafts(draft.items);
    const totalValue = Number(draft.totalValue);
    if (draft.type === "simple" && draft.totalValue.trim() && !Number.isFinite(totalValue)) {
      throw new Error("Total must be a number.");
    }
    const savedPo = await savePurchasePo({
      id: draft.id,
      project: draft.project,
      vendor: draft.vendor,
      poNumber: draft.poNumber,
      type: draft.type,
      totalValue: Number.isFinite(totalValue) ? totalValue : 0,
      items,
      goodsReceived: draft.goodsReceived,
      taxInvoiceNo: draft.taxInvoiceNo,
      taxInvoiceDate: draft.taxInvoiceDate,
    });
    const next = fromPo(savedPo);
    setDraft(next);
    setSaved(next);
    await reload();
    return savedPo;
  }, [draft, reload]);

  const persist = useCallback(async () => {
    await writeBill();
  }, [writeBill]);

  useRegisterUnsaved(dirty, persist);

  async function openEdit(id: number) {
    setBusy(true);
    try {
      const po = await getPurchasePo(id);
      const next = fromPo(po);
      setDraft(next);
      setSaved(next);
      setMode("edit");
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  function openNew() {
    const next = blankDraft();
    setDraft(next);
    setSaved(next);
    setMode("edit");
  }

  async function onDelete(id: number) {
    setBusy(true);
    try {
      await deletePurchasePo(id);
      await reload();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSaveStay() {
    setBusy(true);
    try {
      await persist();
      setMode("list");
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSubmitBill() {
    setBusy(true);
    try {
      const po = await writeBill();
      const out = await submitOffice("purchase", po.poNumber);
      if (out.kind === "conflict") setError(out.message);
      else await reload();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSubmitPay(payNumber: string) {
    if (!payNumber.trim()) {
      setError("Save the payment first so it has a PAY number.");
      return;
    }
    setBusy(true);
    try {
      const out = await submitOffice("payment", payNumber);
      if (out.kind === "conflict") setError(out.message);
      else await reload();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function onAddPay() {
    if (!draft.poNumber) {
      setError("Save the bill first so it has a PUR number.");
      return;
    }
    const amount = Number(payAmount);
    if (!Number.isFinite(amount) || amount <= 0) {
      setError("Payment amount must be greater than zero.");
      return;
    }
    setBusy(true);
    try {
      await savePurchasePayment({
        id: null,
        payNumber: "",
        poNumber: draft.poNumber,
        vendor: draft.vendor,
        amountRupees: amount,
        allocMethod: "",
        payDate: "",
        remarks: "",
      });
      setPayAmount("");
      await reload();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  const filtered = useMemo(() => {
    const list = rows ?? [];
    if (!filters.length) return list;
    return list.filter((row) => {
      const pays = payments.filter((p) => p.poNumber.toLowerCase() === row.poNumber.toLowerCase());
      if (!pays.length) {
        return paymentMatchesFilters(
          { pay_class: "advance", missing_tax_invoice: Boolean(!row.taxInvoiceNo && !row.taxInvoiceDate) },
          filters,
        );
      }
      return pays.some((p) =>
        paymentMatchesFilters({ pay_class: p.payClass, missing_tax_invoice: p.missingTaxInvoice }, filters),
      );
    });
  }, [rows, payments, filters]);

  const billPays = payments.filter((p) => p.poNumber.toLowerCase() === draft.poNumber.toLowerCase());

  if (mode === "edit") {
    return (
      <ModuleFrame
        title={draft.id ? "Edit purchase bill" : "New purchase bill"}
        hint="Save writes this PC only. Submit sends one hive row. Blank number becomes PUR-0001."
        onBack={() => requestLeave(() => setMode("list"))}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void onSaveStay()} disabled={busy}>
              {busy ? "Saving…" : "Save"}
            </Button>
            <Button onClick={() => void onSubmitBill()} disabled={busy}>
              Submit
            </Button>
          </div>
        }
      >
        <div className="rounded-lg bg-paper-raised p-5 ring-1 ring-line">
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Bill</p>
          <div className="mt-3 flex gap-2">
            {(["contract", "simple"] as const).map((kind) => (
              <button
                key={kind}
                type="button"
                className={cn(
                  "pressable h-11 rounded-md px-4 text-sm capitalize",
                  draft.type === kind ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted",
                )}
                onClick={() => setDraft({ ...draft, type: kind })}
              >
                {kind}
              </button>
            ))}
          </div>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <SuggestField
              id="pur-project"
              label="Project"
              required
              value={draft.project}
              onChange={(project) => setDraft({ ...draft, project })}
              options={projects}
            />
            <SuggestField
              id="pur-vendor"
              label="Vendor"
              required
              value={draft.vendor}
              onChange={(vendor) => setDraft({ ...draft, vendor })}
              options={vendors}
            />
            <div>
              <Label htmlFor="pur-po">PUR number</Label>
              <Input
                id="pur-po"
                className="mt-1.5"
                placeholder="Allocated on save if blank"
                value={draft.poNumber}
                onChange={(e) => setDraft({ ...draft, poNumber: e.target.value })}
              />
            </div>
            {draft.type === "simple" ? (
              <div>
                <Label htmlFor="pur-total">Total</Label>
                <Input
                  id="pur-total"
                  className="mt-1.5"
                  inputMode="decimal"
                  value={draft.totalValue}
                  onChange={(e) => setDraft({ ...draft, totalValue: e.target.value })}
                />
              </div>
            ) : null}
            <div>
              <Label htmlFor="pur-tax">Tax invoice</Label>
              <Input
                id="pur-tax"
                className="mt-1.5"
                value={draft.taxInvoiceNo}
                onChange={(e) => setDraft({ ...draft, taxInvoiceNo: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="pur-tax-date">Tax invoice date</Label>
              <Input
                id="pur-tax-date"
                className="mt-1.5"
                value={draft.taxInvoiceDate}
                onChange={(e) => setDraft({ ...draft, taxInvoiceDate: e.target.value })}
              />
            </div>
          </div>
          <label className="mt-4 flex h-11 items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={draft.goodsReceived}
              onChange={(e) => setDraft({ ...draft, goodsReceived: e.target.checked })}
            />
            Goods received
          </label>
          {draft.type === "contract" ? (
            <>
              <div className="ledger-rule my-5" />
              <PoItemsEditor items={draft.items} onChange={(items) => setDraft({ ...draft, items })} />
            </>
          ) : null}
          {draft.poNumber ? (
            <>
              <div className="ledger-rule my-5" />
              <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">PAY payments</p>
              <div className="mt-3 flex flex-col gap-2 sm:flex-row">
                <Input
                  inputMode="decimal"
                  placeholder="Amount"
                  value={payAmount}
                  onChange={(e) => setPayAmount(e.target.value)}
                />
                <Button variant="secondary" onClick={() => void onAddPay()} disabled={busy}>
                  Add PAY
                </Button>
              </div>
              <ul className="mt-3 space-y-2 text-sm">
                {billPays.map((p) => (
                  <li key={p.id} className="flex items-center justify-between rounded-md bg-paper-sunken px-3 py-2">
                    <span className="font-mono">
                      {p.payNumber} · {formatRupees(p.amountRupees)} · {p.payClass}
                    </span>
                    <div className="flex gap-1">
                      <Button size="sm" variant="secondary" onClick={() => void onSubmitPay(p.payNumber)} disabled={busy}>
                        Submit
                      </Button>
                      <Button size="sm" variant="ghost" onClick={() => void deletePurchasePayment(p.id!).then(reload)}>
                        Delete
                      </Button>
                    </div>
                  </li>
                ))}
                {billPays.length === 0 ? <li className="text-ink-muted">No PAY rows on this bill yet.</li> : null}
              </ul>
            </>
          ) : null}
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Purchase"
      hint="PUR bills and PAY bank payments on this PC. Refresh does not change these rows."
      onBack={() => requestLeave(onBack)}
      error={error}
      actions={<Button onClick={openNew}>New bill</Button>}
    >
      <div className="mb-4 flex flex-wrap gap-2">
        {FILTERS.map((f) => {
          const on = filters.includes(f.key);
          return (
            <button
              key={f.key}
              type="button"
              className={cn(
                "pressable h-11 rounded-md px-3 text-sm",
                on ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted",
              )}
              onClick={() =>
                setFilters(on ? filters.filter((k) => k !== f.key) : [...filters, f.key])
              }
            >
              {f.label}
            </button>
          );
        })}
      </div>
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={filtered}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening purchase bills on this PC…" : "No purchase bills on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>PUR</TableHeadCell>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell>Status</TableHeadCell>
              <TableHeadCell className="text-right">Total</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell className="font-mono">{row.poNumber}</TableCell>
              <TableCell className="text-ink-muted">{row.vendor || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.payStatus || "Unpaid"}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
              <TableCell>
                <div className="flex justify-end gap-2">
                  <Button size="sm" variant="secondary" onClick={() => void openEdit(row.id)}>
                    Edit
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => void onDelete(row.id)} disabled={busy}>
                    Delete
                  </Button>
                </div>
              </TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}
