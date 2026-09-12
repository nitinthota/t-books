import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import type { DirtyKey } from "@/lib/t-books/types";

export function ConflictModal({
  voucherNumbers,
  keys,
  busy,
  onKeepLocal,
  onDiscard,
}: {
  voucherNumbers: number[];
  keys?: DirtyKey[] | null;
  busy: boolean;
  onKeepLocal: () => void;
  onDiscard: () => void;
}) {
  const keepRef = useRef<HTMLButtonElement>(null);
  const list =
    keys && keys.length
      ? keys.map((k) => `${k.kind} ${k.key}`)
      : voucherNumbers.map((n) => `voucher ${n}`);

  useEffect(() => {
    keepRef.current?.focus();
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) onKeepLocal();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onKeepLocal]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-4"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onKeepLocal();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="conflict-title"
        className="enter w-full max-w-md rounded-lg bg-paper-raised p-6 ring-1 ring-line"
      >
        <h2 id="conflict-title" className="text-2xl font-medium tracking-tight">
          Unsynced local changes
        </h2>
        <p className="mt-3 text-sm text-ink-muted">You have local edits on these rows:</p>
        <ul className="mt-3 max-h-48 overflow-auto rounded-md bg-paper-sunken px-3 py-2 text-sm">
          {list.map((n) => (
            <li key={n} className="py-1 font-mono text-ink">
              {n}
            </li>
          ))}
        </ul>
        <p className="mt-3 text-sm text-ink-muted">Choose:</p>
        <ul className="mt-1 list-disc pl-5 text-sm text-ink-muted">
          <li>Keep local (cancel refresh)</li>
          <li>Discard local and refresh</li>
        </ul>
        <div className="mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end">
          <Button ref={keepRef} variant="secondary" onClick={onKeepLocal} disabled={busy}>
            Keep Local
          </Button>
          <Button variant="danger" onClick={onDiscard} disabled={busy}>
            Discard Local & Refresh
          </Button>
        </div>
      </div>
    </div>
  );
}
