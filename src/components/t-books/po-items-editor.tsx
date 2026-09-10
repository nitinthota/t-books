import { Plus, Trash2 } from "lucide-react";
import { useMemo } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { formatRupees } from "@/lib/t-books/business_rules";
import { previewPo } from "@/lib/t-books/office";
import type { PoItemIn } from "@/lib/t-books/types";

export type ItemDraft = {
  key: string;
  description: string;
  qty: string;
  rate: string;
  gstPct: string;
};

export function emptyItem(): ItemDraft {
  return { key: `${Date.now()}-${Math.random()}`, description: "", qty: "1", rate: "0", gstPct: "18" };
}

export function draftsFromItems(items: PoItemIn[]): ItemDraft[] {
  if (!items.length) return [emptyItem()];
  return items.map((item) => ({
    key: String(item.id ?? `${item.description}-${item.qty}`),
    description: item.description,
    qty: String(item.qty ?? 0),
    rate: String(item.rate ?? 0),
    gstPct: String(item.gstPct ?? 0),
  }));
}

export function parseItemDrafts(drafts: ItemDraft[]): PoItemIn[] {
  return drafts.map((row) => {
    const qty = Number(row.qty);
    const rate = Number(row.rate);
    const gstPct = Number(row.gstPct);
    if (row.qty.trim() && !Number.isFinite(qty)) throw new Error("Quantity must be a number.");
    if (row.rate.trim() && !Number.isFinite(rate)) throw new Error("Rate must be a number.");
    if (row.gstPct.trim() && !Number.isFinite(gstPct)) throw new Error("GST % must be a number.");
    return {
      description: row.description,
      qty: Number.isFinite(qty) ? qty : 0,
      rate: Number.isFinite(rate) ? rate : 0,
      gstPct: Number.isFinite(gstPct) ? gstPct : 0,
      amount: 0,
    };
  });
}

export function PoItemsEditor({
  items,
  onChange,
}: {
  items: ItemDraft[];
  onChange: (items: ItemDraft[]) => void;
}) {
  const parsed = useMemo(() => {
    try {
      return { ok: true as const, items: parseItemDrafts(items) };
    } catch (err) {
      return {
        ok: false as const,
        message: err instanceof Error ? err.message : "Items are not valid.",
        items: [] as PoItemIn[],
      };
    }
  }, [items]);
  const preview = parsed.ok ? previewPo(parsed.items) : null;

  function patch(index: number, partial: Partial<ItemDraft>) {
    onChange(items.map((row, i) => (i === index ? { ...row, ...partial } : row)));
  }

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Items</p>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => onChange([...items, emptyItem()])}
        >
          <Plus className="size-3.5" strokeWidth={1.75} />
          Add row
        </Button>
      </div>
      <div className="overflow-x-auto rounded-lg bg-paper shadow-[0_0_0_1px_var(--color-line)]">
        <table className="w-full min-w-[40rem] text-left text-sm">
          <thead className="border-b border-line text-xs font-medium uppercase tracking-wide text-ink-subtle">
            <tr>
              <th className="px-3 py-2">Description</th>
              <th className="px-3 py-2">Qty</th>
              <th className="px-3 py-2">Rate</th>
              <th className="px-3 py-2">GST %</th>
              <th className="px-3 py-2 text-right">Amount</th>
              <th className="px-3 py-2" />
            </tr>
          </thead>
          <tbody>
            {items.map((row, index) => (
              <tr key={row.key} className="table-row">
                <td className="px-2 py-2">
                  <Input
                    value={row.description}
                    onChange={(e) => patch(index, { description: e.target.value })}
                    placeholder="Line"
                  />
                </td>
                <td className="w-24 px-2 py-2">
                  <Input
                    inputMode="decimal"
                    value={row.qty}
                    onChange={(e) => patch(index, { qty: e.target.value })}
                    aria-label="Quantity"
                  />
                </td>
                <td className="w-28 px-2 py-2">
                  <Input
                    inputMode="decimal"
                    value={row.rate}
                    onChange={(e) => patch(index, { rate: e.target.value })}
                    aria-label="Rate"
                  />
                </td>
                <td className="w-24 px-2 py-2">
                  <Input
                    inputMode="decimal"
                    value={row.gstPct}
                    onChange={(e) => patch(index, { gstPct: e.target.value })}
                    aria-label="GST percent"
                  />
                </td>
                <td className="px-3 py-2 text-right tabular-nums text-ink-muted">
                  {preview ? formatRupees(preview.items[index]?.amount ?? 0) : "—"}
                </td>
                <td className="px-2 py-2">
                  <button
                    type="button"
                    className="pressable inline-flex size-11 items-center justify-center rounded-md text-ink-muted hover:bg-paper-sunken hover:text-danger"
                    aria-label="Remove row"
                    onClick={() => onChange(items.filter((_, i) => i !== index))}
                    disabled={items.length <= 1}
                  >
                    <Trash2 className="size-4" strokeWidth={1.75} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {!parsed.ok ? (
        <p className="mt-2 text-sm text-danger-fg" role="alert">
          {parsed.message}
        </p>
      ) : null}
      <dl className="mt-4 grid grid-cols-3 gap-3 text-sm">
        <div className="rounded-lg bg-paper-sunken px-3 py-2">
          <dt className="text-xs uppercase tracking-wide text-ink-subtle">Subtotal</dt>
          <dd className="mt-1 font-medium tabular-nums">{formatRupees(preview?.subtotal ?? 0)}</dd>
        </div>
        <div className="rounded-lg bg-paper-sunken px-3 py-2">
          <dt className="text-xs uppercase tracking-wide text-ink-subtle">GST</dt>
          <dd className="mt-1 font-medium tabular-nums">{formatRupees(preview?.gst ?? 0)}</dd>
        </div>
        <div className="rounded-lg bg-navy-soft px-3 py-2">
          <dt className="text-xs uppercase tracking-wide text-navy">Total</dt>
          <dd className="money-figure mt-1 text-lg text-navy">
            {formatRupees(preview?.grandTotal ?? 0)}
          </dd>
        </div>
      </dl>
    </div>
  );
}
