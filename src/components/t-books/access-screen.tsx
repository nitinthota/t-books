import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { canOpenAccess } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
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
    </ModuleFrame>
  );
}
