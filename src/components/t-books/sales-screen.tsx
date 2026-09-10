import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatRupees } from "@/lib/t-books/business_rules";
import {
  deleteSalesPo,
  getSalesPo,
  listProjects,
  listSalesPo,
  saveSalesPo,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { SalesPo } from "@/lib/t-books/types";
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
  items: ItemDraft[];
};

function blankDraft(): Draft {
  return { id: null, project: "", poNumber: "", client: "", gst: "", items: [emptyItem()] };
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

  const persist = useCallback(async () => {
    const items = parseItemDrafts(draft.items);
    const savedPo = await saveSalesPo({
      id: draft.id,
      project: draft.project,
      poNumber: draft.poNumber,
      client: draft.client,
      gst: draft.gst,
      items,
    });
    setDraft(fromPo(savedPo));
    setSaved(fromPo(savedPo));
    await reload();
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

  function openNew() {
    const next = blankDraft();
    setDraft(next);
    setSaved(next);
    setMode("edit");
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

  if (mode === "edit") {
    return (
      <ModuleFrame
        title={draft.id ? "Edit sales PO" : "New sales PO"}
        hint="Contract PO. Totals come from calcPo. Negative qty or rate is a deduction."
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
          <div className="mt-3 grid gap-4 sm:grid-cols-2">
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
          <p className="mb-3 text-xs font-medium uppercase tracking-wide text-ink-subtle">Contract</p>
          <PoItemsEditor items={draft.items} onChange={(items) => setDraft({ ...draft, items })} />
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Sales"
      hint="Contract POs on this PC. Refresh from Google does not change these rows."
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
              {rows === null
                ? "Opening sales POs on this PC…"
                : "No sales POs on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>PO</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell>Client</TableHeadCell>
              <TableHeadCell className="text-right">Total</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.poNumber}</TableCell>
              <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.client || "—"}</TableCell>
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

function fromPo(po: SalesPo): Draft {
  return {
    id: po.id,
    project: po.project,
    poNumber: po.poNumber,
    client: po.client,
    gst: po.gst,
    items: draftsFromItems(po.items),
  };
}
