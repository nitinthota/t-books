import { MessageCircleQuestion } from "lucide-react";
import { useState } from "react";
import { askBooks, type AskResult } from "@/lib/t-books/ask-books";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

const EXAMPLES = [
  "How many bills are pending in 2025 and June 2026?",
  "How many vouchers still to pay in 2024?",
  "Pending purchase bills in 2025",
];

/** Board Ask: English question → local data → English answer. Replaces name search. */
export function AskBooks() {
  const [question, setQuestion] = useState("");
  const [result, setResult] = useState<AskResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);

  async function run(q: string) {
    const text = q.trim();
    if (!text) return;
    setBusy(true);
    setError(null);
    setOpen(true);
    try {
      const out = await askBooks(text);
      setResult(out);
    } catch (err) {
      setResult(null);
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="relative min-w-0 max-w-xl flex-1">
      <MessageCircleQuestion
        className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-ink-subtle"
        strokeWidth={1.75}
        aria-hidden="true"
      />
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void run(question);
        }}
      >
        <input
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          onFocus={() => setOpen(true)}
          placeholder="Ask in English — e.g. pending bills in June 2026"
          aria-label="Ask books a data question"
          className="h-9 w-full rounded-md bg-paper-raised pl-8 pr-3 text-sm text-ink shadow-[0_0_0_1px_var(--color-line)] placeholder:text-ink-subtle focus-visible:outline-none focus-visible:shadow-[0_0_0_2px_var(--color-navy)]"
        />
        <Button type="submit" size="sm" disabled={busy || !question.trim()}>
          {busy ? "…" : "Ask"}
        </Button>
      </form>
      {open ? (
        <div className="absolute right-0 z-30 mt-1 w-full min-w-[20rem] max-w-xl overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
          {error ? (
            <p className="px-3 py-3 text-sm text-danger-fg" role="alert">
              {error}
            </p>
          ) : result ? (
            <div className="px-3 py-3">
              <p className="whitespace-pre-wrap text-sm text-ink">{result.answer}</p>
            </div>
          ) : (
            <div className="px-3 py-3">
              <p className="text-xs text-ink-subtle">Examples</p>
              <ul className="mt-2 space-y-1">
                {EXAMPLES.map((ex) => (
                  <li key={ex}>
                    <button
                      type="button"
                      className={cn(
                        "pressable w-full rounded-md px-2 py-1.5 text-left text-sm text-ink-muted hover:bg-paper-sunken hover:text-ink",
                      )}
                      onClick={() => {
                        setQuestion(ex);
                        void run(ex);
                      }}
                    >
                      {ex}
                    </button>
                  </li>
                ))}
              </ul>
              <p className="mt-2 text-xs text-ink-subtle">
                Answers use this PC’s books only — not Google, not guessed numbers.
              </p>
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
}
