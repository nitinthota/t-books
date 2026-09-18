import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees, paymentOrdinal } from "@/lib/t-books/business_rules";
import { SUBMIT_SUCCESS_TOAST } from "@/lib/t-books/constants";
import { invokeCommand, invokeErrorMessage } from "@/lib/t-books/platform";
import { canMutate, canRefresh } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { PaymentView, VoucherListRow, VoucherSave, VoucherView } from "@/lib/t-books/types";
import { loadVoucher, loadVoucherList, reloadVoucher, submitVoucher } from "@/lib/t-books/vouchers";
import { ModuleFrame } from "./module-frame";
import { MoneyHero } from "./money-hero";
import { RefreshToast } from "./refresh-toast";
import { SubmitConflictModal } from "./submit-conflict";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

function emptyPay(slot: number): PaymentView {
  return {
    slot,
    piNo: "",
    piDate: "",
    piValue: 0,
    paid: 0,
    remaining: 0,
    paymentDate: "",
    remarks: "",
  };
}

function draftFrom(view: VoucherView): VoucherSave {
  const pays = [...view.payments];
  while (pays.length < 5) pays.push(emptyPay(pays.length + 1));
  return {
    voucherNumber: view.voucherNumber,
    taxInvoice: view.taxInvoice,
    vendor: view.vendor,
    gst: view.gst,
    project: view.project,
    comments: view.comments,
    voucherDate: view.voucherDate ?? "",
    bank: view.bank ?? "",
    accountNumber: view.accountNumber ?? "",
    ifsc: view.ifsc ?? "",
    payments: pays.slice(0, 5).map((p, i) => ({ ...p, slot: p.slot || i + 1 })),
  };
}

export default function VouchersScreen({
  onBack,
  focusId,
}: {
  onBack: () => void;
  focusId?: string | null;
}) {
  const session = useBooks((s) => s.session);
  const canSubmit = session ? canRefresh(session.role) : false;
  const canEdit = session ? canMutate(session.role) : false;
  const [rows, setRows] = useState<VoucherListRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<VoucherView | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<VoucherSave | null>(null);
  const [query, setQuery] = useState("");
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
      const view = await loadVoucher(n);
      setOpen(view);
      setDraft(draftFrom(view));
      setEditing(false);
      setError(null);
      setSubmitState("idle");
      setConflictMessage(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    }
  }

  function startNew() {
    const next = (rows ?? []).reduce((m, r) => Math.max(m, r.voucherNumber), 0) + 1;
    const view: VoucherView = {
      voucherNumber: next,
      taxInvoice: "",
      vendor: "",
      project: "",
      gst: "",
      comments: "",
      totalValue: 0,
      totalPaid: 0,
      remaining: 0,
      status: "",
      isDirty: true,
      payments: [],
    };
    setOpen(view);
    setDraft(draftFrom(view));
    setEditing(true);
    setError(null);
  }

  async function onSave() {
    if (!draft || submitState === "submitting") return;
    if (!draft.vendor.trim()) {
      setError("Vendor is required.");
      return;
    }
    setSubmitState("submitting");
    setError(null);
    try {
      const view = await invokeCommand<VoucherView>("save_voucher", { payload: draft });
      setOpen(view);
      setDraft(draftFrom(view));
      setEditing(false);
      setSubmitState("idle");
      await reload();
    } catch (err) {
      setSubmitState("error");
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
      setDraft(draftFrom(view));
      setEditing(false);
      setConflictMessage(null);
      setSubmitState("idle");
      setError(null);
      await reload();
    } catch (err) {
      setSubmitState("error");
      setError(invokeErrorMessage(err));
    }
  }

  const visible = useMemo(() => {
    const list = rows ?? [];
    const q = query.trim().toLowerCase();
    if (!q) return list;
    return list.filter((row) =>
      [
        String(row.voucherNumber),
        row.vendor,
        row.project,
        row.taxInvoice,
        row.status,
      ].some((v) => v.toLowerCase().includes(q)),
    );
  }, [rows, query]);

  if (open && draft) {
    return (
      <ModuleFrame
        title={`Voucher ${open.voucherNumber}`}
        hint={editing ? "Save writes this PC only (dirty). Submit writes Google." : open.status || undefined}
        onBack={() => {
          setOpen(null);
          setEditing(false);
        }}
        error={error}
        actions={
          <>
            {canEdit && !editing ? (
              <Button variant="secondary" onClick={() => setEditing(true)}>
                Edit
              </Button>
            ) : null}
            {canEdit && editing ? (
              <Button onClick={() => void onSave()} disabled={submitState === "submitting"}>
                {submitState === "submitting" ? "Saving…" : "Save on this PC"}
              </Button>
            ) : null}
            {canSubmit && !editing ? (
              <Button onClick={() => void onSubmit()} disabled={submitState === "submitting"}>
                {submitState === "submitting" ? "Submitting…" : "Submit to Google"}
              </Button>
            ) : null}
          </>
        }
      >
        {editing ? (
          <EditForm draft={draft} onChange={setDraft} />
        ) : (
          <>
            <MoneyHero value={open.totalValue} paid={open.totalPaid} remaining={open.remaining} />
            <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2">
              <Meta label="Vendor" value={open.vendor} />
              <Meta label="Project" value={open.project} />
              <Meta label="Tax invoice" value={open.taxInvoice} />
              <Meta label="GST" value={open.gst} />
            </dl>
            {open.comments ? <p className="mt-4 text-sm text-ink-muted">{open.comments}</p> : null}
            <div className="mt-6 overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
              <VirtualTable
                rows={open.payments}
                rowKey={(row) => row.slot}
                empty={<p className="text-sm text-ink-muted">No payments on this voucher.</p>}
                header={
                  <tr>
                    <TableHeadCell>Payment</TableHeadCell>
                    <TableHeadCell>PI</TableHeadCell>
                    <TableHeadCell className="text-right">Value</TableHeadCell>
                    <TableHeadCell className="text-right">Paid</TableHeadCell>
                    <TableHeadCell className="text-right">Still to pay</TableHeadCell>
                    <TableHeadCell>Date</TableHeadCell>
                  </tr>
                }
                renderRow={(row) => {
                  const occupied =
                    (row.paid ?? 0) !== 0 ||
                    (row.piValue ?? 0) !== 0 ||
                    Boolean(row.paymentDate) ||
                    Boolean(row.piNo);
                  const seq = occupied
                    ? open.payments.filter((p) => {
                        const on =
                          (p.paid ?? 0) !== 0 ||
                          (p.piValue ?? 0) !== 0 ||
                          Boolean(p.paymentDate) ||
                          Boolean(p.piNo);
                        return on && p.slot <= row.slot;
                      }).length
                    : 0;
                  return (
                    <tr className="table-row">
                      <TableCell className="font-medium">
                        {occupied ? paymentOrdinal(seq) : `Block ${row.slot}`}
                      </TableCell>
                      <TableCell>{row.piNo || "—"}</TableCell>
                      <TableCell className="text-right tabular-nums">{formatRupees(row.piValue)}</TableCell>
                      <TableCell className="text-right tabular-nums">{formatRupees(row.paid)}</TableCell>
                      <TableCell className="text-right tabular-nums text-navy">
                        {formatRupees(row.remaining)}
                      </TableCell>
                      <TableCell className="text-ink-muted">{row.paymentDate || "—"}</TableCell>
                    </tr>
                  );
                }}
              />
            </div>
          </>
        )}
        <p className="mt-4 text-xs text-ink-subtle">
          Save = this PC + dirty. Submit writes the bill to Voucher register and each occupied
          payment as 1st / 2nd / … on Loopbooks — Payments. Max five payments. Column A stays an integer.
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
      hint="Refresh pulls the archive. Edit/Save stays on this PC. Submit writes Google."
      onBack={onBack}
      error={error}
      actions={
        canEdit ? (
          <Button variant="secondary" onClick={startNew}>
            New voucher
          </Button>
        ) : null
      }
    >
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search number, vendor, project, invoice…"
        aria-label="Search vouchers"
        className="mb-4 h-11 w-full max-w-md rounded-md bg-paper-raised px-3 text-sm ring-1 ring-line"
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.voucherNumber}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null
                ? "Opening the register on this PC…"
                : query.trim()
                  ? "No voucher matches that search on this PC."
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

function Field({
  label,
  value,
  onChange,
  required,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
}) {
  return (
    <label className="block text-sm">
      <span className="text-ink-muted">
        {label}
        {required ? <span className="text-danger"> *</span> : null}
      </span>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="mt-1.5 h-11 w-full rounded-md bg-paper-raised px-3 ring-1 ring-line"
      />
    </label>
  );
}

function EditForm({ draft, onChange }: { draft: VoucherSave; onChange: (next: VoucherSave) => void }) {
  const pays = draft.payments ?? [];
  return (
    <div className="grid gap-4">
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="Vendor" required value={draft.vendor} onChange={(vendor) => onChange({ ...draft, vendor })} />
        <Field label="Project" value={draft.project ?? ""} onChange={(project) => onChange({ ...draft, project })} />
        <Field
          label="Tax invoice"
          value={draft.taxInvoice ?? ""}
          onChange={(taxInvoice) => onChange({ ...draft, taxInvoice })}
        />
        <Field label="GST" value={draft.gst ?? ""} onChange={(gst) => onChange({ ...draft, gst })} />
        <Field
          label="Comments"
          value={draft.comments ?? ""}
          onChange={(comments) => onChange({ ...draft, comments })}
        />
      </div>
      <div className="overflow-x-auto rounded-lg ring-1 ring-line">
        <table className="min-w-full text-sm">
          <thead className="bg-paper-sunken text-left text-xs uppercase tracking-wide text-ink-subtle">
            <tr>
              <th className="px-3 py-2">Payment</th>
              <th className="px-3 py-2">PI</th>
              <th className="px-3 py-2">Value</th>
              <th className="px-3 py-2">Paid</th>
              <th className="px-3 py-2">Date</th>
              <th className="px-3 py-2">Remarks</th>
            </tr>
          </thead>
          <tbody>
            {pays.map((row, i) => (
              <tr key={row.slot || i} className="border-t border-line">
                <td className="px-3 py-2">{paymentOrdinal(i + 1)}</td>
                <td className="px-2 py-1">
                  <input
                    value={row.piNo}
                    onChange={(e) => {
                      const payments = pays.map((p, idx) => (idx === i ? { ...p, piNo: e.target.value } : p));
                      onChange({ ...draft, payments });
                    }}
                    className="h-10 w-24 rounded-md bg-paper-raised px-2 ring-1 ring-line"
                  />
                </td>
                <td className="px-2 py-1">
                  <input
                    type="number"
                    value={row.piValue || ""}
                    onChange={(e) => {
                      const payments = pays.map((p, idx) =>
                        idx === i ? { ...p, piValue: Number(e.target.value) || 0 } : p,
                      );
                      onChange({ ...draft, payments });
                    }}
                    className="h-10 w-24 rounded-md bg-paper-raised px-2 ring-1 ring-line"
                  />
                </td>
                <td className="px-2 py-1">
                  <input
                    type="number"
                    value={row.paid || ""}
                    onChange={(e) => {
                      const payments = pays.map((p, idx) =>
                        idx === i ? { ...p, paid: Number(e.target.value) || 0 } : p,
                      );
                      onChange({ ...draft, payments });
                    }}
                    className="h-10 w-24 rounded-md bg-paper-raised px-2 ring-1 ring-line"
                  />
                </td>
                <td className="px-2 py-1">
                  <input
                    value={row.paymentDate}
                    onChange={(e) => {
                      const payments = pays.map((p, idx) =>
                        idx === i ? { ...p, paymentDate: e.target.value } : p,
                      );
                      onChange({ ...draft, payments });
                    }}
                    className="h-10 w-28 rounded-md bg-paper-raised px-2 ring-1 ring-line"
                  />
                </td>
                <td className="px-2 py-1">
                  <input
                    value={row.remarks}
                    onChange={(e) => {
                      const payments = pays.map((p, idx) => (idx === i ? { ...p, remarks: e.target.value } : p));
                      onChange({ ...draft, payments });
                    }}
                    className="h-10 w-full min-w-32 rounded-md bg-paper-raised px-2 ring-1 ring-line"
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
