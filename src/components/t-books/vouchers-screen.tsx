import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { formatRupees, paymentOrdinal } from "@/lib/t-books/business_rules";
import { SUBMIT_SUCCESS_TOAST } from "@/lib/t-books/constants";
import { invokeCommand, invokeErrorMessage } from "@/lib/t-books/platform";
import { canMutate, canRefresh } from "@/lib/t-books/rbac";
import { useBooks } from "@/lib/t-books/store";
import type { PaymentView, VoucherListRow, VoucherSave, VoucherView } from "@/lib/t-books/types";
import { loadVoucher, loadVoucherList, reloadVoucher, submitVoucher } from "@/lib/t-books/vouchers";
import { deskLine, deskMark } from "@/lib/t-books/desk-line";
import { DeskChips } from "./desk-chips";
import { DeskSearch } from "./desk-search";
import { ModuleFrame } from "./module-frame";
import { RefreshToast } from "./refresh-toast";
import { SubmitConflictModal } from "./submit-conflict";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

type VoucherFilter = "all" | "due" | "partial" | "settled" | "dirty";
type ObjectFacet = "overview" | "payments";

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

function payOccupied(row: PaymentView): boolean {
  return (
    (row.paid ?? 0) !== 0 ||
    (row.piValue ?? 0) !== 0 ||
    Boolean(row.paymentDate) ||
    Boolean(row.piNo)
  );
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
  const [filter, setFilter] = useState<VoucherFilter>("all");
  const [facet, setFacet] = useState<ObjectFacet>("overview");
  const [submitState, setSubmitState] = useState<
    "idle" | "submitting" | "success" | "conflict" | "error"
  >("idle");
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
      setFacet("overview");
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
    setFacet("overview");
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

  const counts = useMemo(() => {
    const list = rows ?? [];
    return {
      all: list.length,
      due: list.filter((r) => (r.remaining ?? 0) > 0.5).length,
      partial: list.filter(
        (r) => (r.totalPaid ?? 0) > 0.5 && (r.remaining ?? 0) > 0.5,
      ).length,
      settled: list.filter(
        (r) => (r.totalValue ?? 0) > 0.5 && (r.remaining ?? 0) <= 0.5,
      ).length,
      dirty: list.filter((r) => Boolean(r.isDirty)).length,
    };
  }, [rows]);

  const visible = useMemo(() => {
    const list = rows ?? [];
    const q = query.trim().toLowerCase();
    return list.filter((row) => {
      if (filter === "due" && !((row.remaining ?? 0) > 0.5)) return false;
      if (filter === "partial" && !((row.totalPaid ?? 0) > 0.5 && (row.remaining ?? 0) > 0.5))
        return false;
      if (filter === "settled" && !((row.totalValue ?? 0) > 0.5 && (row.remaining ?? 0) <= 0.5))
        return false;
      if (filter === "dirty" && !row.isDirty) return false;
      if (!q) return true;
      return [String(row.voucherNumber), row.vendor, row.project, row.taxInvoice, row.status].some(
        (v) => (v || "").toLowerCase().includes(q),
      );
    });
  }, [rows, query, filter]);

  if (open && draft) {
    const statusLine = deskLine(open) ?? open.status ?? "";
    const paymentCount = open.payments.filter(payOccupied).length;
    return (
      <ModuleFrame
        title={`Voucher ${open.voucherNumber}`}
        onBack={() => {
          setOpen(null);
          setEditing(false);
          setFacet("overview");
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
                {submitState === "submitting" ? "Saving\u2026" : "Save draft"}
              </Button>
            ) : null}
            {canSubmit && !editing ? (
              <Button onClick={() => void onSubmit()} disabled={submitState === "submitting"}>
                {submitState === "submitting" ? "Posting\u2026" : "Post"}
              </Button>
            ) : null}
          </>
        }
      >
        <div className="grid gap-3 sm:grid-cols-4">
          <Kpi label="Value" value={formatRupees(open.totalValue)} />
          <Kpi label="Paid" value={formatRupees(open.totalPaid)} />
          <Kpi label="Still to pay" value={formatRupees(open.remaining)} accent />
          <Kpi label="Payments" value={String(paymentCount)} />
        </div>

        {statusLine ? (
          <p className="mt-4 text-sm text-ink-muted">{statusLine}</p>
        ) : null}

        {!editing ? (
          <div className="mt-5 flex flex-wrap gap-2" role="tablist" aria-label="Voucher sections">
            {(
              [
                ["overview", "Overview"],
                ["payments", `Payments ${paymentCount}`],
              ] as const
            ).map(([id, label]) => {
              const on = facet === id;
              return (
                <button
                  key={id}
                  type="button"
                  role="tab"
                  aria-selected={on}
                  onClick={() => setFacet(id)}
                  className={
                    on
                      ? "pressable h-9 rounded-full bg-navy px-3 text-xs text-white"
                      : "pressable h-9 rounded-full bg-paper-raised px-3 text-xs text-ink-muted ring-1 ring-line"
                  }
                >
                  {label}
                </button>
              );
            })}
          </div>
        ) : null}

        {editing ? (
          <div className="mt-6">
            <EditForm draft={draft} onChange={setDraft} />
          </div>
        ) : facet === "overview" ? (
          <dl className="mt-6 grid gap-3 text-sm sm:grid-cols-2">
            <Meta label="Vendor" value={open.vendor} />
            <Meta label="Project" value={open.project} />
            <Meta label="Tax invoice" value={open.taxInvoice} />
            <Meta label="GST" value={open.gst} />
            <Meta label="Status" value={open.status || deskMark(open) || "\u2014"} />
            {open.comments ? <Meta label="Comments" value={open.comments} /> : null}
          </dl>
        ) : (
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
                const occupied = payOccupied(row);
                const seq = occupied
                  ? open.payments.filter((p) => payOccupied(p) && p.slot <= row.slot).length
                  : 0;
                return (
                  <tr className="table-row">
                    <TableCell className="font-medium">
                      {occupied ? paymentOrdinal(seq) : `Block ${row.slot}`}
                    </TableCell>
                    <TableCell>{row.piNo || "\u2014"}</TableCell>
                    <TableCell className="text-right tabular-nums">
                      {formatRupees(row.piValue)}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">
                      {formatRupees(row.paid)}
                    </TableCell>
                    <TableCell className="text-right tabular-nums text-navy">
                      {formatRupees(row.remaining)}
                    </TableCell>
                    <TableCell className="text-ink-muted">{row.paymentDate || "\u2014"}</TableCell>
                  </tr>
                );
              }}
            />
          </div>
        )}

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
      <DeskSearch
        value={query}
        onChange={setQuery}
        placeholder="Number, vendor, project, invoice"
        label="Search vouchers"
      />
      <DeskChips
        label="Voucher filters"
        value={filter}
        onChange={setFilter}
        items={[
          { id: "all", label: "All", count: counts.all },
          { id: "due", label: "Still to pay", count: counts.due },
          { id: "partial", label: "Partial", count: counts.partial },
          { id: "settled", label: "Settled", count: counts.settled },
          { id: "dirty", label: "Not posted", count: counts.dirty },
        ]}
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.voucherNumber}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null
                ? "Opening vouchers\u2026"
                : query.trim() || filter !== "all"
                  ? "No voucher matches."
                  : "No vouchers yet."}
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
                <span className="tabular-nums font-medium">{row.voucherNumber}</span>
                {deskMark(row) ? (
                  <span className="ml-2 text-xs text-gold">{deskMark(row)}</span>
                ) : null}
              </TableCell>
              <TableCell>{row.vendor || "\u2014"}</TableCell>
              <TableCell className="text-ink-muted">{row.project || "\u2014"}</TableCell>
              <TableCell className="text-right tabular-nums">
                {formatRupees(row.totalValue)}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {formatRupees(row.totalPaid)}
              </TableCell>
              <TableCell className="text-right tabular-nums text-navy">
                {formatRupees(row.remaining)}
              </TableCell>
              <TableCell className="text-ink-muted">{row.status || "\u2014"}</TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}

const Kpi = memo(function Kpi({
  label,
  value,
  accent,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className={`money-figure mt-1 text-2xl ${accent ? "text-navy" : ""}`}>{value}</p>
    </div>
  );
});

const Meta = memo(function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-ink-subtle">{label}</dt>
      <dd className="mt-1 text-ink">{value || "\u2014"}</dd>
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
        <Field
          label="Vendor"
          required
          value={draft.vendor}
          onChange={(vendor) => onChange({ ...draft, vendor })}
        />
        <Field
          label="Project"
          value={draft.project ?? ""}
          onChange={(project) => onChange({ ...draft, project })}
        />
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
                      const payments = pays.map((p, idx) =>
                        idx === i ? { ...p, piNo: e.target.value } : p,
                      );
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
                      const payments = pays.map((p, idx) =>
                        idx === i ? { ...p, remarks: e.target.value } : p,
                      );
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
