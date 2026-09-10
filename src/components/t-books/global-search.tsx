import { Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { searchOffice } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { NavId, SearchHit } from "@/lib/t-books/types";
import { cn } from "@/lib/utils";

const KIND_NAV: Record<string, NavId> = {
  vendor: "vendors",
  project: "projects",
  inventory: "inventory",
  logistics: "logistics",
};

export function GlobalSearch({
  onOpen,
}: {
  onOpen: (nav: NavId, hit: SearchHit) => void;
}) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const boxRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const q = query.trim();
    if (q.length < 2) {
      setHits([]);
      setError(null);
      return;
    }
    const handle = window.setTimeout(() => {
      void searchOffice(q)
        .then((rows) => {
          setHits(rows);
          setError(null);
          setOpen(true);
        })
        .catch((err) => {
          setHits([]);
          setError(invokeErrorMessage(err));
          setOpen(true);
        });
    }, 160);
    return () => window.clearTimeout(handle);
  }, [query]);

  useEffect(() => {
    function onDoc(event: MouseEvent) {
      if (!boxRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  return (
    <div ref={boxRef} className="relative min-w-0 max-w-xs flex-1">
      <Search
        className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-ink-subtle"
        strokeWidth={1.75}
        aria-hidden="true"
      />
      <input
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
        }}
        onFocus={() => {
          if (hits.length || error) setOpen(true);
        }}
        placeholder="Search vendors, projects…"
        aria-label="Search vendors, projects, inventory, logistics"
        className="h-9 w-full rounded-md bg-paper-raised pl-8 pr-3 text-sm text-ink shadow-[0_0_0_1px_var(--color-line)] placeholder:text-ink-subtle focus-visible:outline-none focus-visible:shadow-[0_0_0_2px_var(--color-navy)]"
      />
      {open && query.trim().length >= 2 ? (
        <div className="absolute right-0 z-30 mt-1 w-80 max-w-[calc(100vw-2rem)] overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
          {error ? (
            <p className="px-3 py-3 text-sm text-danger-fg" role="alert">
              {error}
            </p>
          ) : hits.length === 0 ? (
            <p className="px-3 py-3 text-sm text-ink-muted">No matches on this PC.</p>
          ) : (
            <ul className="max-h-80 overflow-auto py-1">
              {hits.map((hit) => (
                <li key={`${hit.kind}-${hit.id}`}>
                  <button
                    type="button"
                    className={cn(
                      "pressable flex w-full flex-col items-start px-3 py-2 text-left hover:bg-paper-sunken",
                    )}
                    onClick={() => {
                      const nav = KIND_NAV[hit.kind];
                      if (nav) onOpen(nav, hit);
                      setOpen(false);
                      setQuery("");
                    }}
                  >
                    <span className="text-xs uppercase tracking-wide text-ink-subtle">{hit.kind}</span>
                    <span className="text-sm text-ink">{hit.title}</span>
                    {hit.subtitle ? (
                      <span className="text-xs text-ink-muted">{hit.subtitle}</span>
                    ) : null}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}
