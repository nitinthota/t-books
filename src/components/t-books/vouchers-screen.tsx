import { memo, useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees } from "@/lib/t-books/business_rules";
import { SUBMIT_SUCCESS_TOAST } from "@/lib/t-books/constants";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import { canRefresh } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { VoucherListRow, VoucherView } from "@/lib/t-books/types";
import { loadVoucher, loadVoucherList, reloadVoucher, submitVoucher } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";
import { MoneyHero } from "./money-hero";
import { RefreshToast } from "./refresh-toast";
import { SubmitConflictModal } from "./submit-conflict";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

export default function VouchersScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const session = useBooks((s) => s.session);
  const canSubmit = session ? canRefresh(session.role) : false;
  const [rows, setRows] = useState<VoucherListRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<VoucherView | null>(null);
  const [submitState, setSubmitState] = useState<"idle" | "submitting" | "success" | "conflict" | "error">(
    "idle",
  );
  const [conflictMessage, setConflictMessage] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setRows(await loadVoucherList());
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      setRows((prev) => prev ?? []);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (!focusId) return;
    const n = Number(focusId);
    if (Number.isFinite(n) && n > 0) void openVoucher(n);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusId]);

  async function openVoucher(n: number) {
    try {
      setOpen(await loadVoucher(n));
      setError(null);
      setSubmitState("idle");
      setConflictMessage(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }

  async function onSubmit() {
    if (!open || submitState === "submitting") return;
    setSubmitState("submitting");
    setError(null);
    try {
      const out = await submitVoucher(open.voucherNumber);
      if (out.kind === "conflict") {
        setConflictMessage(out.message);
        setSubmitState("conflict");
        return;
      }
      setSubmitState("success");
      await openVoucher(open.voucherNumber);
      await reload();
    } catch (err) {
      setSubmitState("error");
      setError(invokeErrorMessage(err));
    }
  }

  async function onReloadFromSheet() {
    if (!open) return;
    setSubmitState("submitting");
    try {
      const view = await reloadVoucher(open.voucherNumber);
      setOpen(view);
      setConflictMessage(null);
      setSubmitState("idle");
      setError(null);
      await reload();
    } catch (err) {
      setSubmitState("error");
      setError(invokeErrorMessage(err));
    }
  }

  if (open) {
    return (
      <ModuleFrame
        title={`Voucher ${open.voucherNumber}`}
        hint={open.status || undefined}
        onBack={() => setOpen(null)}
        error={error}
        actions={
          canSubmit ? (
            <Button onClick={() => void onSubmit()} disabled={submitState === "submitting"}>
              {submitState === "submitting" ? "Submitting…" : "Submit to Google"}
            </Button>
          ) : null
        }
      >
        <MoneyHero value={open.totalValue} paid={open.totalPaid} remaining={open.remaining} />
        <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2">
          <Meta label="Vendor" value={open.vendor} />
          <Meta label="Project" value={open.project} />
          <Meta label="Tax invoice" value={open.taxInvoice} />
          <Meta label="GST" value={open.gst} />
        </dl>
        {open.comments ? (
          <p className="mt-4 text-sm text-ink-muted">{open.comments}</p>
        ) : null}
        <div className="mt-6 overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
          <VirtualTable
            rows={open.payments}
            rowKey={(row) => row.slot}
            empty={<p className="text-sm text-ink-muted">No payments on this voucher.</p>}
            header={
              <tr>
                <TableHeadCell>PI</TableHeadCell>
                <TableHeadCell className="text-right">Value</TableHeadCell>
                <TableHeadCell className="text-right">Paid</TableHeadCell>
                <TableHeadCell className="text-right">Still to pay</TableHeadCell>
                <TableHeadCell>Date</TableHeadCell>
              </tr>
            }
            renderRow={(row) => (
              <tr className="table-row">
                <TableCell>{row.piNo || `Block ${row.slot}`}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.piValue)}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.paid)}</TableCell>
                <TableCell className="text-right tabular-nums text-navy">
                  {formatRupees(row.remaining)}
                </TableCell>
                <TableCell className="text-ink-muted">{row.paymentDate || "—"}</TableCell>
              </tr>
            )}
          />
        </div>
        <p className="mt-4 text-xs text-ink-subtle">
          Payments stay on this voucher. Up to five blocks.
        </p>
        {submitState === "success" ? (
          <RefreshToast message={SUBMIT_SUCCESS_TOAST} onDone={() => setSubmitState("idle")} />
        ) : null}
        {submitState === "conflict" && conflictMessage ? (
          <SubmitConflictModal
            message={conflictMessage}
            busy={false}
            onCancel={() => {
              setSubmitState("idle");
              setConflictMessage(null);
            }}
            onReload={() => void onReloadFromSheet()}
          />
        ) : null}
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="Vouchers"
      hint="Local register. Refresh pulls Voucher_Raw_Data. Submit writes one voucher back, and never overwrites another person’s changes."
      onBack={onBack}
      error={error}
    >
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={rows ?? []}
          rowKey={(row) => row.voucherNumber}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null
                ? "Opening the register on this PC…"
                : "No vouchers on this PC yet. Refresh when online."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>#</TableHeadCell>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell>Project</TableHeadCell>
              <TableHeadCell className="text-right">Value</TableHeadCell>
              <TableHeadCell className="text-right">Paid</TableHeadCell>
              <TableHeadCell className="text-right">Still to pay</TableHeadCell>
              <TableHeadCell>Status</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr
              className="table-row cursor-pointer"
              onClick={() => void openVoucher(row.voucherNumber)}
            >
              <TableCell>
                <span className="tabular-nums">{row.voucherNumber}</span>
                {row.isDirty ? <span className="ml-2 text-xs text-gold">unsynced</span> : null}
              </TableCell>
              <TableCell>{row.vendor || "—"}</TableCell>
              <TableCell className="text-ink-muted">{row.project || "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.totalValue)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatRupees(row.totalPaid)}</TableCell>
              <TableCell className="text-right tabular-nums text-navy">
                {formatRupees(row.remaining)}
              </TableCell>
              <TableCell className="text-ink-muted">{row.status || "—"}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}

const Meta = memo(function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-ink-subtle">{label}</dt>
      <dd className="mt-1 text-ink">{value || "—"}</dd>
    </div>
  );
});
