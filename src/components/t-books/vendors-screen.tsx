import { useCallback, useEffect, useState } from "react";
import { listVendors } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { VendorRef } from "@/lib/t-books/types";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function VendorsScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const [rows, setRows] = useState<VendorRef[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setRows(await listVendors());
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
      title="Vendors"
      hint="From this PC. Refresh does not delete these names."
      onBack={onBack}
      error={error}
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row) => row.vendor}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null ? "Opening vendors on this PC…" : "No vendors on this PC yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell>GST</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className={row.vendor === focused ? "table-row bg-navy-soft" : "table-row"}>
              <TableCell>{row.vendor}</TableCell>
              <TableCell className="text-ink-muted">{row.gst || "—"}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}
