import { useCallback, useEffect, useState } from "react";
import { listProjects } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function ProjectsScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const [rows, setRows] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setRows(await listProjects());
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      setRows((prev) => prev ?? []);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const focused = focusId?.trim() ?? "";

  return (
    <ModuleFrame
      title="Projects"
      hint="Touched when you save a PO, voucher, or inventory move."
      onBack={onBack}
      error={error}
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row) => row}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening projects on this PC…" : "No projects on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Project</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className={row === focused ? "table-row bg-navy-soft" : "table-row"}>
              <TableCell>{row}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}
