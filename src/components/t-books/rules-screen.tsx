import { useCallback, useEffect, useState } from "react";
import { listRules, toggleRule, type RuleRow } from "@/lib/t-books/control";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { cn } from "@/lib/utils";
import { ModuleFrame } from "./module-frame";

export default function RulesScreen({ onBack }: { onBack: () => void }) {
  const [rows, setRows] = useState<RuleRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setRows(await listRules());
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function onToggle(row: RuleRow) {
    setBusy(true);
    try {
      await toggleRule(row.id, !row.enabled);
      await load();
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <ModuleFrame
      title="Rules"
      hint="When a voucher type posts, ledger, stock, and project effects fire. Disable a rule to stop that effect for new work on this PC."
      onBack={onBack}
      error={error}
    >
      <ul className="divide-y divide-line overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        {rows.map((r) => (
          <li key={r.id} className="flex items-center justify-between gap-3 px-4 py-3">
            <div>
              <p className="text-sm font-medium">{r.name}</p>
              <p className="text-xs text-ink-muted">
                WHEN {r.whenType} THEN {r.thenAction}
              </p>
            </div>
            <button
              type="button"
              disabled={busy}
              className={cn(
                "pressable h-11 min-w-20 rounded-md px-3 text-sm",
                r.enabled ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted",
              )}
              onClick={() => void onToggle(r)}
            >
              {r.enabled ? "On" : "Off"}
            </button>
          </li>
        ))}
      </ul>
    </ModuleFrame>
  );
}
