import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { writeAccessRow } from "@/lib/t-books/access";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { cn } from "@/lib/utils";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export function AccessScreen({ onBack }: { onBack: () => void }) {
  const session = useBooks((s) => s.session);
  const online = useBooks((s) => s.online);
  const accessRows = useBooks((s) => s.accessRows);
  const lastSynced = useBooks((s) => s.lastSynced);
  const lastError = useBooks((s) => s.lastError);
  const loadAccessList = useBooks((s) => s.loadAccessList);
  const refreshAccess = useBooks((s) => s.refreshAccess);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [form, setForm] = useState({ name: "", email: "", role: "operator", active: "Yes" });

  useEffect(() => {
    void loadAccessList();
  }, [loadAccessList]);

  if (!session || !canOpenAccess(session.role)) return null;

  async function onRefresh() {
    setActionError(null);
    setBusy(true);
    try {
      const result = await refreshAccess();
      if (!result.ok) setActionError(result.message);
    } finally {
      setBusy(false);
    }
  }

  const errorText = actionError ?? lastError;

  return (
    <ModuleFrame
      title="Access"
      hint={lastSynced ? `Last synced ${lastSynced}` : "Never synced on this PC."}
      onBack={onBack}
      error={errorText}
      actions={
        <Button onClick={() => void onRefresh()} disabled={busy || !online}>
          <RefreshCw
            className={cn("size-4", busy ? "animate-spin" : "")}
            strokeWidth={1.75}
            aria-hidden="true"
          />
          {busy ? "Refreshing Access…" : "Refresh"}
        </Button>
      }
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={accessRows}
          rowKey={(row) => row.email}
          empty={
            <p className="text-sm text-ink-muted">
              No Access rows on this PC yet. Refresh pulls the Access tab from Google.
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Name</TableHeadCell>
              <TableHeadCell>Email</TableHeadCell>
              <TableHeadCell>Role</TableHeadCell>
              <TableHeadCell>Active</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.name || "—"}</TableCell>
              <TableCell>{row.email}</TableCell>
              <TableCell className="capitalize text-ink-muted">{row.role || "—"}</TableCell>
              <TableCell>
                <span
                  className={cn(
                    "inline-flex h-7 items-center rounded-md px-2 text-xs font-medium",
                    row.active === "Yes" ? "bg-navy-soft text-navy" : "bg-danger-bg text-danger-fg",
                  )}
                >
                  {row.active || "—"}
                </span>
              </TableCell>
            </tr>
          )}
        />
      </div>
      <div className="mt-6 rounded-lg bg-paper-raised p-5 ring-1 ring-line">
        <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Add or disable</p>
        <p className="mt-1 text-sm text-ink-muted">Writes one Access row. Owner only. This PC must be online.</p>
        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          <div>
            <Label htmlFor="acc-name">Name</Label>
            <Input
              id="acc-name"
              className="mt-1.5"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="acc-email">Email</Label>
            <Input
              id="acc-email"
              className="mt-1.5"
              value={form.email}
              onChange={(e) => setForm({ ...form, email: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="acc-role">Role</Label>
            <select
              id="acc-role"
              className="mt-1.5 h-11 w-full rounded-md bg-paper-raised px-3 text-base"
              value={form.role}
              onChange={(e) => setForm({ ...form, role: e.target.value })}
            >
              <option value="operator">operator</option>
              <option value="admin">admin</option>
            </select>
          </div>
          <label className="flex h-11 items-center gap-2 self-end text-sm">
            <input
              type="checkbox"
              checked={form.active === "Yes"}
              onChange={(e) => setForm({ ...form, active: e.target.checked ? "Yes" : "No" })}
            />
            Active
          </label>
        </div>
        <Button
          className="mt-4"
          disabled={busy || !online}
          onClick={() => {
            setBusy(true);
            setActionError(null);
            void writeAccessRow(form)
              .then(() => loadAccessList())
              .catch((err) => setActionError(invokeErrorMessage(err)))
              .finally(() => setBusy(false));
          }}
        >
          Submit Access row
        </Button>
      </div>
    </ModuleFrame>
  );
}
