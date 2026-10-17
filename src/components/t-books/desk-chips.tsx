/** One chip bar. Use this on every desk. Do not invent a second chip style. */

export function DeskChips<T extends string>({
  label,
  value,
  onChange,
  items,
}: {
  label: string;
  value: T;
  onChange: (id: T) => void;
  items: { id: T; label: string; count: number }[];
}) {
  return (
    <div className="mb-4 flex flex-wrap gap-2" role="tablist" aria-label={label}>
      {items.map((item) => {
        const on = value === item.id;
        return (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={on}
            onClick={() => onChange(item.id)}
            className={
              on
                ? "pressable h-9 rounded-full bg-navy px-3 text-xs text-white"
                : "pressable h-9 rounded-full bg-paper-raised px-3 text-xs text-ink-muted ring-1 ring-line"
            }
          >
            {item.label} {item.count}
          </button>
        );
      })}
    </div>
  );
}
