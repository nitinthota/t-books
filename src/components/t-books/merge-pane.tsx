import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  canUnmergeVoucher,
  lastVoucherUnmerge,
  mergeBlockedReason,
  pickDefaultMergeTarget,
  vendorMergeKey,
  type MergeVoucherHint,
} from "@/lib/t-books/business_rules";
import type { PurchasePo, VoucherListRow } from "@/lib/t-books/types";

export function MergePane({
  bills,
  vouchers,
}: {
  bills: PurchasePo[];
  vouchers: VoucherListRow[];
}) {
  const [selected, setSelected] = useState<number[]>([]);
  const [targetPo, setTargetPo] = useState("");

  const hints: MergeVoucherHint[] = useMemo(
    () =>
      vouchers.map((row) => ({
        id: String(row.voucherNumber),
        vendor_key: vendorMergeKey(row.vendor, null),
        po_id: bills.find((b) => b.vendor.trim().toLowerCase() === row.vendor.trim().toLowerCase())?.poNumber ?? null,
        source: row.status.toLowerCase().includes("import") ? "import" : "purchase",
        reversed: false,
      })),
    [vouchers, bills],
  );

  const chosen = hints.filter((h) => selected.includes(Number(h.id)));
  const targetBill = bills.find((b) => b.poNumber === targetPo) ?? null;
  const blocked = mergeBlockedReason(
    chosen,
    targetBill?.poNumber ?? null,
    targetBill ? vendorMergeKey(targetBill.vendor, null) : null,
  );
  const suggested = pickDefaultMergeTarget({
    voucherPoIds: chosen.map((h) => h.po_id),
    vendorPurchaseIds: bills
      .filter((b) => chosen[0] && vendorMergeKey(b.vendor, null) === chosen[0].vendor_key)
      .map((b) => b.poNumber),
  });

  const unmergeable = chosen.filter((h) => canUnmergeVoucher(h.source));
  const lastAction = lastVoucherUnmerge(chosen.length - 1, {
    autoDelete: false,
    itemCount: targetBill?.items.length ?? 0,
    hasPostedPayments: Boolean(targetBill?.paidRupees && targetBill.paidRupees > 0),
  });

  function toggle(n: number) {
    setSelected((cur) => (cur.includes(n) ? cur.filter((x) => x !== n) : [...cur, n]));
  }

  return (
    <section className="mt-6 rounded-lg bg-paper-raised p-5 ring-1 ring-line">
      <p className="text-xs font-medium uppercase tracking-wide text-ink-subtle">Merge onto a bill</p>
      <p className="mt-1 text-sm text-ink-muted">
        Same vendor only. Posted PUR-n / PUR-n-01 / PAY-n are never rewritten. This pane reads SQLite on this PC.
      </p>
      <div className="mt-3 flex flex-wrap gap-2">
        {vouchers.slice(0, 40).map((row) => {
          const on = selected.includes(row.voucherNumber);
          return (
            <button
              key={row.voucherNumber}
              type="button"
              className={`pressable h-11 rounded-md px-3 font-mono text-sm ${
                on ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted"
              }`}
              onClick={() => toggle(row.voucherNumber)}
            >
              {row.voucherNumber} · {row.vendor || "—"}
            </button>
          );
        })}
        {vouchers.length === 0 ? <p className="text-sm text-ink-muted">No vouchers on this PC to merge.</p> : null}
      </div>
      <label className="mt-4 block text-sm">
        Target PUR
        <select
          className="mt-1.5 h-11 w-full rounded-md bg-paper-sunken px-3"
          value={targetPo}
          onChange={(e) => setTargetPo(e.target.value)}
        >
          <option value="">{suggested.kind === "new" ? "New bill (default)" : `Suggested ${suggested.po_id}`}</option>
          {bills.map((b) => (
            <option key={b.id} value={b.poNumber}>
              {b.poNumber} · {b.vendor}
            </option>
          ))}
        </select>
      </label>
      <p className={`mt-3 text-sm ${blocked ? "text-debit" : "text-ink-muted"}`}>
        {blocked ??
          (chosen.length
            ? `Ready: ${chosen.length} voucher(s) can sit on ${targetPo || "a new bill"}. Unmerge last action: ${lastAction}.`
            : "Select at least one voucher.")}
      </p>
      <p className="mt-1 text-xs text-ink-subtle">
        Unmerge is only for linked import payments ({unmergeable.length} selected).
      </p>
      <Button className="mt-3" variant="secondary" disabled={Boolean(blocked) || chosen.length === 0}>
        Rules OK — keep working on the bill editor
      </Button>
    </section>
  );
}
