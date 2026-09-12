import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { calcPo, formatRupees, paiseToRupees, paymentTermDays } from "@/lib/t-books/business_rules";
import {
  deleteSalesPo,
  getSalesPo,
  listProjects,
  listSalesPo,
  previewPo,
  saveSalesPo,
  submitOffice,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { SalesPo, SalesPoKind } from "@/lib/t-books/types";
import { cn } from "@/lib/utils";
import { ModuleFrame, SuggestField } from "./module-frame";
import { draftsFromItems, emptyItem, parseItemDrafts, PoItemsEditor, type ItemDraft } from "./po-items-editor";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";
import { useRegisterUnsaved, useUnsaved } from "./unsaved-guard";

type Draft = {
  id: number | null;
  project: string;
  poNumber: string;
  client: string;
  gst: string;
  kind: SalesPoKind;
  received: string;
  termValue: string;
  termUnit: "days" | "months";
  items: ItemDraft[];
};

function blankDraft(): Draft {
  return {
    id: null,
    project: "",
    poNumber: "",
    client: "",
    gst: "",
    kind: "contract",
    received: "0",
    termValue: "0",
    termUnit: "days",
    items: [emptyItem()],
  };
}

function fromPo(po: SalesPo): Draft {
  return {
    id: po.id,
    project: po.project,
    poNumber: po.poNumber,
    client: po.client,
    gst: po.gst,
    kind: po.kind === "project" ? "project" : "contract",
    received: String(po.receivedRupees ?? 0),
    termValue: String(po.paymentTermValue ?? 0),
    termUnit: po.paymentTermUnit === "months" ? "months" : "days",
    items: draftsFromItems(po.items ?? []),
  };
}

function salesKey(poNumber: string, project: string): string {
  return `${poNumber.trim()}@${project.trim()}`;
}

export default function SalesScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const { requestLeave } = useUnsaved();
  const [rows, setRows] = useState<SalesPo[] | null>(null);
  const [projects, setProjects] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "edit">("list");
  const [draft, setDraft] = useState<Draft>(blankDraft);
  const [saved, setSaved] = useState<Draft>(blankDraft);
  const [busy, setBusy] = useState(false);
  const dirty = mode === "edit" && JSON.stringify(draft) !== JSON.stringify(saved);

  const reload = useCallback(async () => {
    try {
      const [list, projectNames] = await Promise.all([listSalesPo(), listProjects()]);
      setRows(list);
      setProjects(projectNames);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      setRows((prev) => prev ?? []);
    }
  }, []);

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

  const summary = useMemo(() => {
    try {
      const items = parseItemDrafts(draft.items);
      const received = Number(draft.received);
      const termValue = Number(draft.termValue);
      return calcPo({
        payment_term_value: Number.isFinite(termValue) ? termValue : 0,
        payment_term_unit: draft.termUnit,
        amount_received_rupees: Number.isFinite(received) ? received : 0,
        items: items.map((item) => ({
          item_name: item.itemName,
          description: item.description,
          qty: item.qty,
          unit_rate_rupees: item.rate,
          gst_pct: item.gstPct,
        })),
      });
    } catch {
      return null;
    }
  }, [draft]);

  const persist = useCallback(async () => {
    const items = parseItemDrafts(draft.items);
    const received = Number(draft.received);
    const termValue = Number(draft.termValue);
    const savedPo = await saveSalesPo({
      id: draft.id,
      project: draft.project,
      poNumber: draft.poNumber,
      client: draft.client,
      gst: draft.gst,
      items,
      kind: draft.kind,
      receivedRupees: Number.isFinite(received) ? received : 0,
      paymentTermValue: Number.isFinite(termValue) ? termValue : 0,
      paymentTermUnit: draft.termUnit,
    });
    setDraft(fromPo(savedPo));
    setSaved(fromPo(savedPo));
    await reload();
    return savedPo;
  }, [draft, reload]);

  useRegisterUnsaved(dirty, persist);

  async function openEdit(id: number) {
    setBusy(true);
    try {
      const po = await getSalesPo(id);
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

  function openNew(kind: SalesPoKind = "contract") {
    const next = { ...blankDraft(), kind };
    setDraft(next);
    setSaved(next);
    setMode("edit");
  }

  function cloneAsProject() {
    if (!draft.project.trim()) {
      setError("Project is required before cloning.");
      return;
    }
    const next: Draft = {
      ...draft,
      id: null,
      kind: "project",
      poNumber: draft.poNumber ? `${draft.poNumber}-P` : "",
    };
    setDraft(next);
    setSaved(blankDraft());
    setError(null);
  }

  async function onDelete(id: number) {
    setBusy(true);
    try {
      await deleteSalesPo(id);
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

  async function onSubmit() {
    setBusy(true);
    try {
      const po = await persist();
      const out = await submitOffice("sales_po", salesKey(po.poNumber, po.project));
      if (out.kind === "conflict") setError(out.message);
      else await reload();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  if (mode === "edit") {
    const grand = summary ? paiseToRupees(summary.grand_total_paise) : previewPo([]).grandTotal;
    const received = summary ? paiseToRupees(summary.amount_received_paise) : 0;
    const balance = summary ? paiseToRupees(summary.balance_paise) : grand;
    return (
      <ModuleFrame
        title={draft.id ? "Edit sales PO" : "New sales PO"}
        hint="Save is this PC + dirty. Submit publishes one Sales_PO hive row keyed PO@project. Same PO on another project is allowed."
        onBack={() => requestLeave(() => setMode("list"))}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void onSaveStay()} disabled={busy}>
              {busy ? "Saving…" : "Save"}
            </Button>
            <Button onClick={() => void onSubmit()} disabled={busy}>
              Submit
            </Button>
          </div>
        }
      >
        <div className="rounded-lg bg-paper-raised p-5 ring-1 ring-line">
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Basic</p>
          <div className="mt-3 flex flex-wrap gap-2">
            {(["contract", "project"] as const).map((kind) => (
              <button
                key={kind}
                type="button"
                className={cn(
                  "pressable h-11 rounded-md px-4 text-sm capitalize",
                  draft.kind === kind ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted",
                )}
                onClick={() => setDraft({ ...draft, kind })}
              >
                {kind}
              </button>
            ))}
            {draft.kind === "contract" ? (
              <Button variant="ghost" onClick={cloneAsProject}>
                Clone to project PO
              </Button>
            ) : null}
          </div>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <SuggestField
              id="sales-project"
              label="Project"
              required
              value={draft.project}
              onChange={(project) => setDraft({ ...draft, project })}
              options={projects}
            />
            <div>
              <Label htmlFor="sales-po">PO number</Label>
              <Input
                id="sales-po"
                className="mt-1.5"
                value={draft.poNumber}
                onChange={(e) => setDraft({ ...draft, poNumber: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="sales-client">Client</Label>
              <Input
                id="sales-client"
                className="mt-1.5"
                value={draft.client}
                onChange={(e) => setDraft({ ...draft, client: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="sales-gst">GST</Label>
              <Input
                id="sales-gst"
                className="mt-1.5"
                value={draft.gst}
                onChange={(e) => setDraft({ ...draft, gst: e.target.value })}
              />
            </div>
          </div>

          <div className="ledger-rule my-5" />
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Contract</p>
          <div className="mt-3 grid gap-4 sm:grid-cols-3">
            <div>
              <Label htmlFor="sales-term">Payment term</Label>
              <Input
                id="sales-term"
                className="mt-1.5"
                inputMode="decimal"
                value={draft.termValue}
                onChange={(e) => setDraft({ ...draft, termValue: e.target.value })}
              />
            </div>
            <div>
              <Label>Unit</Label>
              <div className="mt-1.5 flex gap-2">
                {(["days", "months"] as const).map((unit) => (
                  <button
                    key={unit}
                    type="button"
                    className={cn(
                      "pressable h-11 rounded-md px-3 text-sm capitalize",
                      draft.termUnit === unit ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted",
                    )}
                    onClick={() => setDraft({ ...draft, termUnit: unit })}
                  >
                    {unit}
                  </button>
                ))}
              </div>
            </div>
            <div>
              <Label htmlFor="sales-received">Received (negatives allowed)</Label>
              <Input
                id="sales-received"
                className="mt-1.5"
                inputMode="decimal"
                value={draft.received}
                onChange={(e) => setDraft({ ...draft, received: e.target.value })}
              />
            </div>
          </div>
          <p className="mt-2 text-xs text-ink-muted">
            Term days: {paymentTermDays(Number(draft.termValue) || 0, draft.termUnit)}
          </p>
          <div className="mt-4">
            <PoItemsEditor items={draft.items} onChange={(items) => setDraft({ ...draft, items })} />
          </div>

          <div className="ledger-rule my-5" />
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Summary</p>
          <dl className="mt-3 grid grid-cols-2 gap-3 text-sm sm:grid-cols-4">
            <div className="rounded-lg bg-paper-sunken px-3 py-2">
              <dt className="text-xs uppercase tracking-wide text-ink-subtle">Subtotal</dt>
              <dd className="mt-1 font-medium tabular-nums">
                {formatRupees(summary ? paiseToRupees(summary.subtotal_paise) : 0)}
              </dd>
            </div>
            <div className="rounded-lg bg-paper-sunken px-3 py-2">
              <dt className="text-xs uppercase tracking-wide text-ink-subtle">GST</dt>
              <dd className="mt-1 font-medium tabular-nums">
                {formatRupees(summary ? paiseToRupees(summary.gst_paise) : 0)}
              </dd>
            </div>
            <div className="rounded-lg bg-paper-sunken px-3 py-2">
              <dt className="text-xs uppercase tracking-wide text-ink-subtle">Received</dt>
              <dd className="mt-1 font-medium tabular-nums">{formatRupees(received)}</dd>
            </div>
            <div className="rounded-lg bg-navy-soft px-3 py-2">
              <dt className="text-xs uppercase tracking-wide text-navy">Grand / balance</dt>
              <dd className="mt-1 text-sm tabular-nums text-navy">
                {formatRupees(grand)} · {formatRupees(balance)}
              </dd>
            </div>
          </dl>
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Sales"
      hint="Contract and project POs on this PC. Refresh from Google does not delete these rows."
      onBack={() => requestLeave(onBack)}
      error={error}
      actions={
        <div className="flex gap-2">
          <Button variant="secondary" onClick={() => openNew("project")}>
            New project PO
          </Button>
          <Button onClick={() => openNew("contract")}>New contract PO</Button>
        </div>
      }
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening sales POs on this PC…" : "No sales POs on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>PO</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell>Kind</TableHeadCell>
              <TableHeadCell>Client</TableHeadCell>
              <TableHeadCell className="text-right">Grand</TableHeadCell>
              <TableHeadCell className="text-right">Received</TableHeadCell>
              <TableHeadCell className="text-right">Balance</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => {
            const received = row.receivedRupees ?? 0;
            return (
              <tr className="table-row">
                <TableCell>
                  {row.poNumber}
                  {row.isDirty ? " · dirty" : ""}
                </TableCell>
                <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
                <TableCell className="text-ink-muted">{row.kind === "project" ? "project" : "contract"}</TableCell>
                <TableCell className="text-ink-muted">{row.client || "—"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(received)}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue - received)}</TableCell>
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
            );
          }}
        />
      </div>
    </ModuleFrame>
  );
}
