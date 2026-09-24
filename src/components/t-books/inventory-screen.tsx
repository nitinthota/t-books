import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatRupees } from "@/lib/t-books/business_rules";
import {
  deleteInventory,
  listInventory,
  listProjects,
  moveInventory,
  saveInventory,
  submitOffice,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { InventoryRow } from "@/lib/t-books/types";
import { ModuleFrame, SuggestField } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";
import { useRegisterUnsaved, useUnsaved } from "./unsaved-guard";

type Draft = {
  id: number | null;
  itemName: string;
  type: string;
  size: string;
  quantity: string;
  cost: string;
  project: string;
};

function blank(): Draft {
  return { id: null, itemName: "", type: "", size: "", quantity: "0", cost: "0", project: "" };
}

export default function InventoryScreen({
  onBack,
  focusId,
  onOpenProject,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpenProject?: (project: string) => void;
}) {
  const { requestLeave } = useUnsaved();
  const [rows, setRows] = useState<InventoryRow[] | null>(null);
  const [projects, setProjects] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "edit" | "move">("list");
  const [draft, setDraft] = useState<Draft>(blank());
  const [saved, setSaved] = useState<Draft>(blank());
  const [moveId, setMoveId] = useState<number | null>(null);
  const [moveProject, setMoveProject] = useState("");
  const [busy, setBusy] = useState(false);
  const dirty = mode === "edit" && JSON.stringify(draft) !== JSON.stringify(saved);

  const reload = useCallback(async (q?: string) => {
    try {
      const [list, projectNames] = await Promise.all([listInventory(q), listProjects()]);
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
  }, [reload]);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      void reload(query);
    }, 160);
    return () => window.clearTimeout(handle);
  }, [query, reload]);

  useEffect(() => {
    if (!focusId) return;
    const id = Number(focusId);
    if (Number.isFinite(id) && id > 0 && rows) {
      const match = rows.find((row) => row.id === id);
      if (match) openEdit(match);
      return;
    }
    setQuery(focusId);
  }, [focusId, rows]);

  const persist = useCallback(async () => {
    const quantity = Number(draft.quantity);
    const cost = Number(draft.cost);
    if (draft.quantity.trim() && !Number.isFinite(quantity)) throw new Error("Quantity must be a number.");
    if (draft.cost.trim() && !Number.isFinite(cost)) throw new Error("Cost must be a number.");
    const savedRow = await saveInventory({
      id: draft.id,
      itemName: draft.itemName,
      type: draft.type,
      size: draft.size,
      quantity: Number.isFinite(quantity) ? quantity : 0,
      cost: Number.isFinite(cost) ? cost : 0,
      project: draft.project,
    });
    const next = fromRow(savedRow);
    setDraft(next);
    setSaved(next);
    await reload(query);
    return savedRow;
  }, [draft, query, reload]);

  useRegisterUnsaved(dirty, async () => {
    await persist();
  });

  function openEdit(row: InventoryRow) {
    const next = fromRow(row);
    setDraft(next);
    setSaved(next);
    setMode("edit");
  }

  async function onSave() {
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

  async function onSubmitHive() {
    setBusy(true);
    try {
      const savedRow = await persist();
      const key = `INV-${savedRow.id}`;
      const out = await submitOffice("inventory", key);
      if (out.kind === "conflict") setError(out.message);
      else {
        setMode("list");
        setError(null);
      }
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function onMoveSave() {
    if (!moveId) return;
    setBusy(true);
    try {
      await moveInventory(moveId, moveProject);
      setMode("list");
      setMoveId(null);
      await reload(query);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  const visible = rows ?? [];
  const jobs = useMemo(
    () => [...new Set(visible.map((row) => row.project.trim()).filter(Boolean))],
    [visible],
  );
  const stockValue = visible.reduce((sum, row) => sum + (row.cost || 0) * (row.quantity || 0), 0);

  if (mode === "edit") {
    return (
      <ModuleFrame
        title={draft.id ? draft.itemName || "Edit item" : "New item"}
        hint="Save writes this PC only. Submit sends one hive row when the Inventory tab exists."
        onBack={() => requestLeave(() => setMode("list"))}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void onSave()} disabled={busy}>
              {busy ? "Saving…" : "Save"}
            </Button>
            <Button onClick={() => void onSubmitHive()} disabled={busy}>
              Submit
            </Button>
          </div>
        }
      >
        <dl className="mb-4 grid gap-3 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-xs uppercase tracking-wide text-ink-subtle">Project</dt>
            <dd className="mt-1">
              {draft.project && onOpenProject ? (
                <button type="button" className="text-navy hover:underline" onClick={() => onOpenProject(draft.project)}>
                  {draft.project}
                </button>
              ) : (
                draft.project || "—"
              )}
            </dd>
          </div>
        </dl>
        <div className="grid gap-4 rounded-lg bg-paper-raised p-5 ring-1 ring-line sm:grid-cols-2">
          <div className="sm:col-span-2">
            <Label htmlFor="inv-name">Item name</Label>
            <Input
              id="inv-name"
              className="mt-1.5"
              value={draft.itemName}
              onChange={(e) => setDraft({ ...draft, itemName: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="inv-type">Type</Label>
            <Input
              id="inv-type"
              className="mt-1.5"
              value={draft.type}
              onChange={(e) => setDraft({ ...draft, type: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="inv-size">Size</Label>
            <Input
              id="inv-size"
              className="mt-1.5"
              value={draft.size}
              onChange={(e) => setDraft({ ...draft, size: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="inv-qty">Quantity</Label>
            <Input
              id="inv-qty"
              className="mt-1.5"
              inputMode="decimal"
              value={draft.quantity}
              onChange={(e) => setDraft({ ...draft, quantity: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="inv-cost">Cost</Label>
            <Input
              id="inv-cost"
              className="mt-1.5"
              inputMode="decimal"
              value={draft.cost}
              onChange={(e) => setDraft({ ...draft, cost: e.target.value })}
            />
          </div>
          <div className="sm:col-span-2">
            <SuggestField
              id="inv-project"
              label="Project"
              value={draft.project}
              onChange={(project) => setDraft({ ...draft, project })}
              options={projects}
              placeholder="Optional"
            />
          </div>
        </div>
      </ModuleFrame>
    );
  }

  if (mode === "move") {
    return (
      <ModuleFrame
        title="Move to project"
        onBack={() => requestLeave(() => setMode("list"))}
        error={error}
        actions={
          <Button onClick={() => void onMoveSave()} disabled={busy}>
            {busy ? "Saving…" : "Move"}
          </Button>
        }
      >
        <div className="rounded-lg bg-paper-raised p-5 ring-1 ring-line">
          <SuggestField
            id="inv-move-project"
            label="Project"
            value={moveProject}
            onChange={setMoveProject}
            options={projects}
          />
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Inventory"
      hint="Stock on this PC. Open a job name to see that job."
      onBack={() => requestLeave(onBack)}
      error={error}
      actions={
        <Button
          onClick={() => {
            const next = blank();
            setDraft(next);
            setSaved(next);
            setMode("edit");
          }}
        >
          New item
        </Button>
      }
    >
      <div className="mb-4 grid gap-3 sm:grid-cols-3">
        <Count label="Items" value={String(visible.length)} />
        <Count label="Jobs" value={String(jobs.length)} />
        <Count label="Stock value" value={formatRupees(stockValue)} />
      </div>
      <Input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Item, type, job"
        aria-label="Search inventory"
        className="mb-4 max-w-sm"
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row, i) => row.id ?? i}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null
                ? "Opening inventory on this PC…"
                : query.trim()
                  ? "No item matches."
                  : "No inventory on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Item</TableHeadCell>
              <TableHeadCell>Type</TableHeadCell>
              <TableHeadCell>Size</TableHeadCell>
              <TableHeadCell className="text-right">Qty</TableHeadCell>
              <TableHeadCell className="text-right">Cost</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.itemName}</TableCell>
              <TableCell className="text-ink-muted">{row.type || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.size || "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{row.quantity}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.cost)}</TableCell>
              <TableCell>
                {row.project && onOpenProject ? (
                  <button
                    type="button"
                    className="text-navy hover:underline"
                    onClick={() => onOpenProject(row.project)}
                  >
                    {row.project}
                  </button>
                ) : (
                  <span className="text-ink-muted">{row.project || "—"}</span>
                )}
              </TableCell>
              <TableCell>
                <div className="flex justify-end gap-2">
                  <Button size="sm" variant="secondary" onClick={() => openEdit(row)}>
                    Edit
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      setMoveId(row.id ?? null);
                      setMoveProject(row.project);
                      setMode("move");
                    }}
                  >
                    Move
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={busy || !row.id}
                    onClick={() => {
                      if (!row.id) return;
                      void (async () => {
                        setBusy(true);
                        try {
                          await deleteInventory(row.id as number);
                          await reload(query);
                        } catch (err) {
                          setError(invokeErrorMessage(err));
                        } finally {
                          setBusy(false);
                        }
                      })();
                    }}
                  >
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

function Count({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className="money-figure mt-1 text-2xl">{value}</p>
    </div>
  );
}

function fromRow(row: InventoryRow): Draft {
  return {
    id: row.id ?? null,
    itemName: row.itemName,
    type: row.type,
    size: row.size,
    quantity: String(row.quantity ?? 0),
    cost: String(row.cost ?? 0),
    project: row.project,
  };
}
