import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";

export function SubmitConflictModal({
  message,
  busy,
  onReload,
  onCancel,
}: {
  message: string;
  busy: boolean;
  onReload: () => void;
  onCancel: () => void;
}) {
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancelRef.current?.focus();
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) onCancel();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onCancel]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-4"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onCancel();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="submit-conflict-title"
        className="enter w-full max-w-md rounded-lg bg-paper-raised p-6 ring-1 ring-line"
      >
        <h2 id="submit-conflict-title" className="text-2xl font-medium tracking-tight">
          Someone else changed this voucher
        </h2>
        <p className="mt-3 whitespace-pre-line text-sm text-ink">{message}</p>
        <div className="mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end">
          <Button ref={cancelRef} variant="secondary" onClick={onCancel} disabled={busy}>
            Cancel
          </Button>
          <Button variant="primary" onClick={onReload} disabled={busy}>
            Reload from sheet
          </Button>
        </div>
      </div>
    </div>
  );
}
