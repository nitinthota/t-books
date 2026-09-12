import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  deleteLogistics,
  listLogistics,
  listProjects,
  moveLogistics,
  saveLogistics,
  submitOffice,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { LogisticsRow } from "@/lib/t-books/types";
import { ModuleFrame, SuggestField } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";
import { useRegisterUnsaved, useUnsaved } from "./unsaved-guard";

type Draft = {
  id: number | null;
  project: string;
  vehicleNumber: string;
  invoiceNumber: string;
  startDate: string;
  reachDate: string;
};

function blank(): Draft {
  return { id: null, project: "", vehicleNumber: "", invoiceNumber: "", startDate: "", reachDate: "" };
}

export default function LogisticsScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const { requestLeave } = useUnsaved();
  const [rows, setRows] = useState<LogisticsRow[] | null>(null);
  const [projects, setProjects] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "edit" | "move">("list");
  const [draft, setDraft] = useState<Draft>(blank());
  const [saved, setSaved] = useState<Draft>(blank());
  const [moveId, setMoveId] = useState<number | null>(null);
  const [moveProject, setMoveProject] = useState("");
  const [busy, setBusy] = useState(false);
  const dirty = mode === "edit" && JSON.stringify(draft) !== JSON.stringify(saved);

  const reload = useCallback(async () => {
    try {
      const [list, projectNames] = await Promise.all([listLogistics(), listProjects()]);
      setRows(list);
      setProjects(projectNames);
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
    if (!focusId || !rows) return;
    const match = rows.find((row) => String(row.id) === focusId);
    if (match) openEdit(match);
  }, [focusId, rows]);

  const persist = useCallback(async () => {
    const savedRow = await saveLogistics({
      id: draft.id,
      project: draft.project,
      vehicleNumber: draft.vehicleNumber,
      invoiceNumber: draft.invoiceNumber,
      startDate: draft.startDate,
      reachDate: draft.reachDate,
    });
    const next = fromRow(savedRow);
    setDraft(next);
    setSaved(next);
    await reload();
    return savedRow;
  }, [draft, reload]);

  useRegisterUnsaved(dirty, async () => {
    await persist();
  });

  function openEdit(row: LogisticsRow) {
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
      const saved = await persist();
      const key = `TRIP-${saved.id}`;
      const out = await submitOffice("logistics", key);
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
      await moveLogistics(moveId, moveProject);
      setMode("list");
      setMoveId(null);
      await reload();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  if (mode === "edit") {
    return (
      <ModuleFrame
        title={draft.id ? "Edit trip" : "New trip"}
        hint="Save writes this PC only. Submit sends one hive row when the Logistics tab exists."
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
        <div className="grid gap-4 rounded-lg bg-paper-raised p-5 ring-1 ring-line sm:grid-cols-2">
          <SuggestField
            id="log-project"
            label="Project"
            value={draft.project}
            onChange={(project) => setDraft({ ...draft, project })}
            options={projects}
          />
          <div>
            <Label htmlFor="log-vehicle">Vehicle number</Label>
            <Input
              id="log-vehicle"
              className="mt-1.5"
              value={draft.vehicleNumber}
              onChange={(e) => setDraft({ ...draft, vehicleNumber: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="log-inv">Invoice number</Label>
            <Input
              id="log-inv"
              className="mt-1.5"
              value={draft.invoiceNumber}
              onChange={(e) => setDraft({ ...draft, invoiceNumber: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="log-start">Start date</Label>
            <Input
              id="log-start"
              className="mt-1.5"
              type="date"
              value={draft.startDate}
              onChange={(e) => setDraft({ ...draft, startDate: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="log-reach">Reach date</Label>
            <Input
              id="log-reach"
              className="mt-1.5"
              type="date"
              value={draft.reachDate}
              onChange={(e) => setDraft({ ...draft, reachDate: e.target.value })}
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
            id="log-move-project"
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
      title="Logistics"
      hint="Trips on this PC. Refresh from Google does not change these rows."
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
          New trip
        </Button>
      }
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row, i) => row.id ?? i}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening logistics on this PC…" : "No trips on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Vehicle</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell>Invoice</TableHeadCell>
              <TableHeadCell>Start</TableHeadCell>
              <TableHeadCell>Reach</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.vehicleNumber}</TableCell>
              <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.invoiceNumber || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.startDate || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.reachDate || "—"}</TableCell>
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
                          await deleteLogistics(row.id as number);
                          await reload();
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

function fromRow(row: LogisticsRow): Draft {
  return {
    id: row.id ?? null,
    project: row.project,
    vehicleNumber: row.vehicleNumber,
    invoiceNumber: row.invoiceNumber,
    startDate: row.startDate,
    reachDate: row.reachDate,
  };
}
