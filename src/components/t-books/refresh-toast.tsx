import { useEffect } from "react";
import { Check } from "lucide-react";

export function RefreshToast({ message, onDone }: { message: string; onDone: () => void }) {
  useEffect(() => {
    const id = window.setTimeout(onDone, 2400);
    return () => window.clearTimeout(id);
  }, [message, onDone]);

  return (
    <div
      role="status"
      className="enter pointer-events-none fixed bottom-6 right-6 z-40 flex items-center gap-2 rounded-md bg-navy px-4 py-3 text-sm text-paper-raised"
    >
      <Check className="size-4" strokeWidth={1.75} aria-hidden="true" />
      {message}
    </div>
  );
}
