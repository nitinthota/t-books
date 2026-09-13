import { useCallback, useEffect, useState } from "react";
import { exploreTable, EXPLORER_TABLES, type ExplorerTable } from "@/lib/t-books/control";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { cn } from "@/lib/utils";
import { ModuleFrame } from "./module-frame";

export default function ExplorerScreen({ onBack }: { onBack: () => void }) {
  const [table, setTable] = useState<ExplorerTable>("vouchers");
  const [rows, setRows] = useState<Array<Record<string, string>>>([]);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (next: ExplorerTable) => {
    try {
      setRows(await exploreTable(next));
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      setRows([]);
    }
  }, []);

  useEffect(() => {
    void load(table);
  }, [load, table]);

  const cols = rows[0] ? Object.keys(rows[0]).slice(0, 8) : [];

  return (
    <ModuleFrame
      title="Explorer"
      hint="Browse local tables on this PC. Password and token columns stay hidden. Changes still go through the modules."
      onBack={onBack}
      error={error}
    >
      <div className="mb-4 flex flex-wrap gap-2">
        {EXPLORER_TABLES.map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTable(t)}
            className={cn(
              "pressable h-10 rounded-full px-3 text-xs",
              table === t ? "bg-navy text-paper-raised" : "bg-paper-raised ring-1 ring-line",
            )}
          >
            {t}
          </button>
        ))}
      </div>
      <div className="overflow-x-auto rounded-lg bg-paper-raised ring-1 ring-line">
        <table className="w-full min-w-[640px] text-sm">
          <thead className="text-left text-xs uppercase tracking-wide text-ink-muted">
            <tr>
              {cols.map((c) => (
                <th key={c} className="px-3 py-2">
                  {c}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td className="px-3 py-6 text-sm text-ink-muted" colSpan={Math.max(cols.length, 1)}>
                  No rows in this table.
                </td>
              </tr>
            ) : (
              rows.map((r, i) => (
                <tr key={i} className="border-t border-line">
                  {cols.map((c) => (
                    <td key={c} className="max-w-[180px] truncate px-3 py-2">
                      {r[c] ?? ""}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </ModuleFrame>
  );
}
