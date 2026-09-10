import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  deleteDocument,
  listDocuments,
  openDocument,
  saveDocument,
} from "@/lib/t-books/office";
import { invokeErrorMessage, isTauriRuntime } from "@/lib/t-books/platform";
import type { DocumentRow } from "@/lib/t-books/types";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";
import { useRegisterUnsaved, useUnsaved } from "./unsaved-guard";

type Draft = {
  id: number | null;
  name: string;
  path: string;
  linkedType: string;
  linkedId: string;
};

function blank(): Draft {
  return { id: null, name: "", path: "", linkedType: "", linkedId: "" };
}

export default function DocumentsScreen({ onBack }: { onBack: () => void }) {
  const { requestLeave } = useUnsaved();
  const [rows, setRows] = useState<DocumentRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "edit">("list");
  const [draft, setDraft] = useState<Draft>(blank());
  const [saved, setSaved] = useState<Draft>(blank());
  const [busy, setBusy] = useState(false);
  const dirty = mode === "edit" && JSON.stringify(draft) !== JSON.stringify(saved);

  const reload = useCallback(async () => {
    try {
      setRows(await listDocuments());
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

  const persist = useCallback(async () => {
    const savedRow = await saveDocument({
      id: draft.id,
      name: draft.name,
      path: draft.path,
      linkedType: draft.linkedType,
      linkedId: draft.linkedId,
      createdAt: "",
    });
    const next = fromRow(savedRow);
    setDraft(next);
    setSaved(next);
    await reload();
  }, [draft, reload]);

  useRegisterUnsaved(dirty, persist);

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

  async function onOpen(id: number) {
    setBusy(true);
    try {
      await openDocument(id);
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
        title={draft.id ? "Edit document" : "Attach document"}
        hint="Only the file path is stored on this PC. T Books does not upload the file."
        onBack={() => requestLeave(() => setMode("list"))}
        error={error}
        actions={
          <Button onClick={() => void onSave()} disabled={busy}>
            {busy ? "Saving…" : "Save"}
          </Button>
        }
      >
        <div className="grid gap-4 rounded-lg bg-paper-raised p-5 ring-1 ring-line sm:grid-cols-2">
          <div className="sm:col-span-2">
            <Label htmlFor="doc-name">Name</Label>
            <Input
              id="doc-name"
              className="mt-1.5"
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </div>
          <div className="sm:col-span-2">
            <Label htmlFor="doc-path">File path</Label>
            <Input
              id="doc-path"
              className="mt-1.5"
              value={draft.path}
              onChange={(e) => setDraft({ ...draft, path: e.target.value })}
              placeholder="%USERPROFILE%\Documents\file.pdf"
            />
            <label className="mt-2 inline-flex h-11 cursor-pointer items-center text-sm text-navy">
              <input
                type="file"
                className="sr-only"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  if (!file) return;
                  setDraft({
                    ...draft,
                    name: draft.name || file.name,
                    path: draft.path || file.name,
                  });
                }}
              />
              Choose a file on this PC
            </label>
          </div>
          <div>
            <Label htmlFor="doc-type">Link type</Label>
            <select
              id="doc-type"
              className="mt-1.5 h-11 w-full rounded-md bg-paper-raised px-3 text-base text-ink shadow-[0_0_0_1px_var(--color-line)] focus-visible:outline-none focus-visible:shadow-[0_0_0_2px_var(--color-navy)]"
              value={draft.linkedType}
              onChange={(e) => setDraft({ ...draft, linkedType: e.target.value })}
            >
              <option value="">None</option>
              <option value="voucher">Voucher</option>
              <option value="project">Project</option>
              <option value="po">PO</option>
            </select>
          </div>
          <div>
            <Label htmlFor="doc-link">Linked id</Label>
            <Input
              id="doc-link"
              className="mt-1.5"
              value={draft.linkedId}
              onChange={(e) => setDraft({ ...draft, linkedId: e.target.value })}
              placeholder="20 or CUST_01 or PO-1"
            />
          </div>
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Documents"
      hint="Paths only — nothing is uploaded. Refresh from Google does not change these rows."
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
          Attach
        </Button>
      }
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row, i) => row.id ?? i}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening documents on this PC…" : "No documents on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Name</TableHeadCell>
              <TableHeadCell>Path</TableHeadCell>
              <TableHeadCell>Link</TableHeadCell>
              <TableHeadCell />
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.name || "—"}</TableCell>
              <TableCell className="max-w-[14rem] truncate text-ink-muted" title={row.path}>
                {row.path || "—"}
              </TableCell>
              <TableCell className="text-ink-muted">
                {row.linkedType ? `${row.linkedType} ${row.linkedId}`.trim() : "—"}
              </TableCell>
              <TableCell>
                <div className="flex justify-end gap-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={busy || !row.id}
                    onClick={() => row.id && void onOpen(row.id)}
                  >
                    {isTauriRuntime() ? "Open file" : "Show path"}
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      const next = fromRow(row);
                      setDraft(next);
                      setSaved(next);
                      setMode("edit");
                    }}
                  >
                    Edit
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
                          await deleteDocument(row.id as number);
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

function fromRow(row: DocumentRow): Draft {
  return {
    id: row.id ?? null,
    name: row.name,
    path: row.path,
    linkedType: row.linkedType,
    linkedId: row.linkedId,
  };
}
