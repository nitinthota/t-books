import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatRupees } from "@/lib/t-books/business_rules";
import {
  deletePurchasePo,
  getPurchasePo,
  listProjects,
  listPurchasePo,
  listVendors,
  savePurchasePo,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { PurchasePo, PurchaseType } from "@/lib/t-books/types";
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
  items: ItemDraft[];
};

function blankDraft(): Draft {
  return {
    id: null,
    project: "",
    vendor: "",
    poNumber: "",
    type: "contract",
    totalValue: "0",
    items: [emptyItem()],
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
  const [projects, setProjects] = useState<string[]>([]);
  const [vendors, setVendors] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "edit">("list");
  const [draft, setDraft] = useState<Draft>(blankDraft);
  const [saved, setSaved] = useState<Draft>(blankDraft);
  const [busy, setBusy] = useState(false);
  const dirty = mode === "edit" && JSON.stringify(draft) !== JSON.stringify(saved);

  const reload = useCallback(async () => {
    try {
      const [list, projectNames, vendorRows] = await Promise.all([
        listPurchasePo(),
        listProjects(),
        listVendors(),
      ]);
      setRows(list);
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

  const persist = useCallback(async () => {
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
    });
    const next = fromPo(savedPo);
    setDraft(next);
    setSaved(next);
    await reload();
  }, [draft, reload]);

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

  if (mode === "edit") {
    return (
      <ModuleFrame
        title={draft.id ? "Edit purchase PO" : "New purchase PO"}
        hint="Contract uses calcPo on items. Simple stores the total you type."
        onBack={() => requestLeave(() => setMode("list"))}
        error={error}
        actions={
          <Button onClick={() => void onSaveStay()} disabled={busy}>
            {busy ? "Saving…" : "Save"}
          </Button>
        }
      >
        <div className="rounded-lg bg-paper-raised p-5 ring-1 ring-line">
          <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Basic</p>
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
              <Label htmlFor="pur-po">PO number</Label>
              <Input
                id="pur-po"
                className="mt-1.5"
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
          </div>
          {draft.type === "contract" ? (
            <>
              <div className="ledger-rule my-5" />
              <p className="mb-3 text-xs font-medium uppercase tracking-wide text-ink-subtle">
                Contract
              </p>
              <PoItemsEditor items={draft.items} onChange={(items) => setDraft({ ...draft, items })} />
            </>
          ) : null}
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Purchase"
      hint="Contract and simple POs on this PC. Refresh from Google does not change these rows."
      onBack={() => requestLeave(onBack)}
      error={error}
      actions={<Button onClick={openNew}>New PO</Button>}
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row) => row.id}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening purchase POs on this PC…" : "No purchase POs on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>PO</TableHeadCell>
              <TableHeadCell>Type</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell className="text-right">Total</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.poNumber}</TableCell>
              <TableCell className="capitalize text-ink-muted">{row.type}</TableCell>
              <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.vendor || "—"}</TableCell>
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

function fromPo(po: PurchasePo): Draft {
  return {
    id: po.id,
    project: po.project,
    vendor: po.vendor,
    poNumber: po.poNumber,
    type: po.type,
    totalValue: String(po.totalValue ?? 0),
    items: draftsFromItems(po.items),
  };
}
