import { useCallback, useEffect, useState } from "react";
import { formatRupees } from "@/lib/t-books/business_rules";
import { getTrial, listFy } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { TrialBalance } from "@/lib/t-books/types";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function TrialScreen({ onBack }: { onBack: () => void }) {
  const [fy, setFy] = useState("");
  const [options, setOptions] = useState<string[]>([]);
  const [trial, setTrial] = useState<TrialBalance | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (nextFy?: string) => {
    try {
      const years = await listFy();
      setOptions(years);
      const chosen = nextFy || fy || years[0] || "";
      if (chosen && chosen !== fy) setFy(chosen);
      const data = await getTrial(chosen || null);
      setTrial(data);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }, [fy]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModuleFrame
      title="Finance"
      hint="Trial balance from posted books on this PC. Indian financial year 1 Apr–31 Mar. Debits must equal credits."
      onBack={onBack}
      error={error}
    >
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <label className="text-sm text-ink-muted">
          FY
          <select
            className="ml-2 h-11 rounded-md bg-paper-sunken px-3 text-ink"
            value={fy || trial?.fy || ""}
            onChange={(e) => {
              setFy(e.target.value);
              void load(e.target.value);
            }}
          >
            {(options.length ? options : trial ? [trial.fy] : []).map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
        </label>
        {trial ? (
          <p className={trial.balanced ? "text-sm text-ink-muted" : "text-sm text-gold"}>
            {trial.balanced ? "Balanced" : "Does not balance"} · {formatRupees(trial.totalDebitRupees)} Dr /{" "}
            {formatRupees(trial.totalCreditRupees)} Cr
          </p>
        ) : null}
      </div>
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={trial?.lines ?? []}
          rowKey={(row) => row.account}
          empty={<p className="text-sm text-ink-muted">No local books in this year yet.</p>}
          header={
            <tr>
              <TableHeadCell>Account</TableHeadCell>
              <TableHeadCell className="text-right">Debit</TableHeadCell>
              <TableHeadCell className="text-right">Credit</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row">
              <TableCell>{row.account}</TableCell>
              <TableCell className="text-right tabular-nums">
                {row.debitRupees ? formatRupees(row.debitRupees) : ""}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {row.creditRupees ? formatRupees(row.creditRupees) : ""}
              </TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}
