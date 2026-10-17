import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatBankLabel, formatRupees } from "@/lib/t-books/business_rules";
import { listPurchasePo, listSalesPo, listVendors } from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { NavId, VendorAccount, VendorRef } from "@/lib/t-books/types";
import { listVendorAccounts, saveVendor, submitVendor } from "@/lib/t-books/vendor-master";
import { loadVoucherList } from "@/lib/t-books/vouchers";
import { DeskChips } from "./desk-chips";
import { DeskSearch } from "./desk-search";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";

type VendorRow = VendorRef & {
  bankLine: string;
  billed: number;
  paid: number;
  due: number;
  vouchers: number;
  purchaseCount: number;
  salesCount: number;
  jobs: string[];
  accounts: VendorAccount[];
};

type VendorFilter = "all" | "due" | "paid" | "no_gst" | "no_bank" | "empty";
type ObjectFacet = "overview" | "banks" | "activity";

function blankAccount(): VendorAccount {
  return { label: "", bank: "", accountNumber: "", ifsc: "", isPrimary: false };
}

export default function VendorsScreen({
  onBack,
  focusId,
  onOpen,
}: {
  onBack: () => void;
  focusId?: string | null;
  onOpen?: (nav: NavId, key: string) => void;
}) {
  const [rows, setRows] = useState<VendorRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<VendorFilter>("all");
  const [openName, setOpenName] = useState<string | null>(focusId?.trim() || null);
  const [gst, setGst] = useState("");
  const [accounts, setAccounts] = useState<VendorAccount[]>([]);
  const [busy, setBusy] = useState(false);
  const [facet, setFacet] = useState<ObjectFacet>("overview");

  const reload = useCallback(async () => {
    try {
      const [vendors, list, bills, sales] = await Promise.all([
        listVendors(),
        loadVoucherList(),
        listPurchasePo(),
        listSalesPo(),
      ]);
      const extra = new Map<string, string>();
      for (const row of list)
        if (row.vendor.trim()) extra.set(row.vendor.trim().toLowerCase(), row.vendor.trim());
      for (const row of bills)
        if (row.vendor.trim()) extra.set(row.vendor.trim().toLowerCase(), row.vendor.trim());
      const names = new Map<string, VendorRef>();
      for (const vendor of vendors) names.set(vendor.vendor.trim().toLowerCase(), vendor);
      for (const [key, label] of extra) {
        if (!names.has(key)) names.set(key, { vendor: label, gst: "" });
      }
      const enriched: VendorRow[] = [];
      for (const vendor of names.values()) {
        const key = vendor.vendor.trim().toLowerCase();
        const v = list.filter((row) => row.vendor.trim().toLowerCase() === key);
        const p = bills.filter((row) => row.vendor.trim().toLowerCase() === key);
        const s = sales.filter((row) => row.client.trim().toLowerCase() === key);
        const jobs = [
          ...new Set([...v, ...p].map((row) => row.project.trim()).filter(Boolean)),
        ].sort((a, b) => a.localeCompare(b));
        let bankAccounts = vendor.accounts ?? [];
        if (!bankAccounts.length) {
          try {
            bankAccounts = await listVendorAccounts(vendor.vendor);
          } catch {
            bankAccounts = [];
          }
        }
        if (!bankAccounts.length && (vendor.bank || vendor.accountNumber || vendor.ifsc)) {
          bankAccounts = [
            {
              label: "Primary",
              bank: vendor.bank || "",
              accountNumber: vendor.accountNumber || "",
              ifsc: vendor.ifsc || "",
              isPrimary: true,
            },
          ];
        }
        const primary = bankAccounts.find((a) => a.isPrimary) ?? bankAccounts[0];
        enriched.push({
          ...vendor,
          accounts: bankAccounts,
          bankLine: formatBankLabel({
            bank_name: primary?.bank || vendor.bank,
            account_no: primary?.accountNumber || vendor.accountNumber,
            ifsc: primary?.ifsc || vendor.ifsc,
          }),
          vouchers: v.length,
          purchaseCount: p.length,
          salesCount: s.length,
          billed: v.reduce((sum, row) => sum + (row.totalValue || 0), 0),
          paid: v.reduce((sum, row) => sum + (row.totalPaid || 0), 0),
          due: v.reduce((sum, row) => sum + (row.remaining || 0), 0),
          jobs,
        });
      }
      enriched.sort((a, b) => a.vendor.localeCompare(b.vendor));
      setRows(enriched);
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
    if (focusId?.trim()) {
      setOpenName(focusId.trim());
      setFacet("overview");
    }
  }, [focusId]);

  const open = (rows ?? []).find((row) => row.vendor === openName) ?? null;

  useEffect(() => {
    if (!open) return;
    setGst(open.gst || "");
    setAccounts(open.accounts.length ? open.accounts.map((a) => ({ ...a })) : [blankAccount()]);
  }, [open]);

  const counts = useMemo(() => {
    const list = rows ?? [];
    return {
      all: list.length,
      due: list.filter((row) => row.due > 0.5).length,
      paid: list.filter((row) => row.billed > 0.5 && row.due <= 0.5).length,
      no_gst: list.filter((row) => !row.gst.trim()).length,
      no_bank: list.filter(
        (row) => !row.bankLine.trim() && row.accounts.every((a) => !a.bank && !a.accountNumber),
      ).length,
      empty: list.filter(
        (row) => row.vouchers === 0 && row.purchaseCount === 0 && row.salesCount === 0,
      ).length,
    };
  }, [rows]);

  const visible = useMemo(() => {
    const list = rows ?? [];
    const q = query.trim().toLowerCase();
    return list.filter((row) => {
      if (filter === "due" && !(row.due > 0.5)) return false;
      if (filter === "paid" && !(row.billed > 0.5 && row.due <= 0.5)) return false;
      if (filter === "no_gst" && row.gst.trim()) return false;
      if (
        filter === "no_bank" &&
        (row.bankLine.trim() || row.accounts.some((a) => a.bank || a.accountNumber))
      )
        return false;
      if (filter === "empty" && (row.vouchers || row.purchaseCount || row.salesCount)) return false;
      if (!q) return true;
      const hay = [
        row.vendor,
        row.gst,
        row.bankLine,
        ...row.jobs,
        ...row.accounts.map((a) => `${a.bank} ${a.accountNumber} ${a.ifsc}`),
      ];
      return hay.some((v) => v.toLowerCase().includes(q));
    });
  }, [rows, query, filter]);

  async function onSaveDraft() {
    if (!open) return;
    setBusy(true);
    try {
      await saveVendor({ vendor: open.vendor, gst, accounts });
      await reload();
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function onPostParty() {
    if (!open) return;
    setBusy(true);
    try {
      await saveVendor({ vendor: open.vendor, gst, accounts });
      const out = await submitVendor(open.vendor);
      if (out.kind === "conflict") {
        setError(out.message);
      } else {
        await reload();
        setError(null);
      }
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  if (open) {
    return (
      <ModuleFrame
        title={open.vendor}
        onBack={() => {
          setOpenName(null);
          setFacet("overview");
        }}
        error={error}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => void onSaveDraft()} disabled={busy}>
              {busy ? "Saving\u2026" : "Save draft"}
            </Button>
            <Button onClick={() => void onPostParty()} disabled={busy}>
              {busy ? "Posting\u2026" : "Post"}
            </Button>
          </div>
        }
      >
        <div className="grid gap-3 sm:grid-cols-4">
          <Kpi label="Billed" value={formatRupees(open.billed)} />
          <Kpi label="Paid" value={formatRupees(open.paid)} />
          <Kpi label="Still to pay" value={formatRupees(open.due)} accent />
          <Kpi label="Banks" value={String(accounts.length)} />
        </div>

        <div className="mt-5 flex flex-wrap gap-2" role="tablist" aria-label="Vendor sections">
          {(
            [
              ["overview", "Overview"],
              ["banks", `Banks ${accounts.length}`],
              ["activity", "Activity"],
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

        {facet === "overview" ? (
          <div className="mt-6 space-y-6">
            <div className="rounded-lg bg-paper-raised p-5 ring-1 ring-line">
              <Label htmlFor="vendor-gst">GST</Label>
              <Input
                id="vendor-gst"
                className="mt-1.5"
                value={gst}
                onChange={(e) => setGst(e.target.value)}
              />
            </div>
            <div className="grid gap-3 sm:grid-cols-3">
              <CountCard
                label="Vouchers"
                value={String(open.vouchers)}
                onClick={() => onOpen?.("vouchers", open.vendor)}
              />
              <CountCard
                label="Purchase bills"
                value={String(open.purchaseCount)}
                onClick={() => onOpen?.("purchase", open.vendor)}
              />
              <CountCard
                label="Sales as client"
                value={String(open.salesCount)}
                onClick={() => onOpen?.("sales", open.vendor)}
              />
            </div>
          </div>
        ) : null}

        {facet === "banks" ? (
          <div className="mt-6">
            <div className="flex items-center justify-between">
              <p className="text-xs uppercase tracking-wide text-ink-subtle">Bank accounts</p>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setAccounts((prev) => [...prev, blankAccount()])}
              >
                Add account
              </Button>
            </div>
            <ul className="mt-3 space-y-3">
              {accounts.map((acc, i) => (
                <li
                  key={`${acc.id ?? "new"}-${i}`}
                  className="rounded-lg bg-paper-raised p-4 ring-1 ring-line"
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div>
                      <Label>Label</Label>
                      <Input
                        className="mt-1.5"
                        value={acc.label}
                        placeholder={i === 0 ? "Primary" : `Account ${i + 1}`}
                        onChange={(e) =>
                          setAccounts((prev) =>
                            prev.map((row, n) =>
                              n === i ? { ...row, label: e.target.value } : row,
                            ),
                          )
                        }
                      />
                    </div>
                    <div>
                      <Label>Bank</Label>
                      <Input
                        className="mt-1.5"
                        value={acc.bank}
                        onChange={(e) =>
                          setAccounts((prev) =>
                            prev.map((row, n) =>
                              n === i ? { ...row, bank: e.target.value } : row,
                            ),
                          )
                        }
                      />
                    </div>
                    <div>
                      <Label>Account number</Label>
                      <Input
                        className="mt-1.5"
                        value={acc.accountNumber}
                        onChange={(e) =>
                          setAccounts((prev) =>
                            prev.map((row, n) =>
                              n === i ? { ...row, accountNumber: e.target.value } : row,
                            ),
                          )
                        }
                      />
                    </div>
                    <div>
                      <Label>IFSC</Label>
                      <Input
                        className="mt-1.5"
                        value={acc.ifsc}
                        onChange={(e) =>
                          setAccounts((prev) =>
                            prev.map((row, n) =>
                              n === i ? { ...row, ifsc: e.target.value } : row,
                            ),
                          )
                        }
                      />
                    </div>
                  </div>
                  <div className="mt-3 flex items-center justify-between">
                    <label className="flex items-center gap-2 text-sm">
                      <input
                        type="radio"
                        name="primary-bank"
                        checked={acc.isPrimary}
                        onChange={() =>
                          setAccounts((prev) =>
                            prev.map((row, n) => ({ ...row, isPrimary: n === i })),
                          )
                        }
                      />
                      Primary
                    </label>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={accounts.length <= 1}
                      onClick={() => setAccounts((prev) => prev.filter((_, n) => n !== i))}
                    >
                      Remove
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        {facet === "activity" ? (
          <div className="mt-6">
            <p className="text-xs uppercase tracking-wide text-ink-subtle">Jobs</p>
            {open.jobs.length === 0 ? (
              <p className="mt-2 text-sm text-ink-muted">No job on this vendor yet.</p>
            ) : (
              <ul className="mt-2 divide-y divide-line rounded-lg bg-paper-raised ring-1 ring-line">
                {open.jobs.map((job) => (
                  <li key={job}>
                    <button
                      type="button"
                      className="pressable w-full px-3 py-2.5 text-left text-sm text-navy"
                      onClick={() => onOpen?.("projects", job)}
                    >
                      {job}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : null}
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame title="Vendors" onBack={onBack} error={error}>
      <DeskSearch
        value={query}
        onChange={setQuery}
        placeholder="Vendor, GST, bank, job"
        label="Search vendors"
      />
      <DeskChips
        label="Vendor filters"
        value={filter}
        onChange={setFilter}
        items={[
          { id: "all", label: "All parties", count: counts.all },
          { id: "due", label: "Still to pay", count: counts.due },
          { id: "paid", label: "Settled", count: counts.paid },
          { id: "no_gst", label: "No GST", count: counts.no_gst },
          { id: "no_bank", label: "No bank", count: counts.no_bank },
          { id: "empty", label: "Name only", count: counts.empty },
        ]}
      />
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        <VirtualTable
          rows={visible}
          rowKey={(row) => row.vendor}
          empty={
            <p className="text-sm text-ink-muted">
              {rows === null
                ? "Opening vendors\u2026"
                : query.trim() || filter !== "all"
                  ? "No vendor matches."
                  : "No vendors yet."}
            </p>
          }
          header={
            <tr>
              <TableHeadCell>Vendor</TableHeadCell>
              <TableHeadCell>GST</TableHeadCell>
              <TableHeadCell>Bank</TableHeadCell>
              <TableHeadCell className="text-right">Vouchers</TableHeadCell>
              <TableHeadCell className="text-right">Still to pay</TableHeadCell>
            </tr>
          }
          renderRow={(row) => (
            <tr className="table-row cursor-pointer" onClick={() => setOpenName(row.vendor)}>
              <TableCell className="font-medium">{row.vendor}</TableCell>
              <TableCell className="text-ink-muted">{row.gst || "\u2014"}</TableCell>
              <TableCell className="text-ink-muted">
                {row.accounts.length > 1
                  ? `${row.bankLine || "\u2014"} \u00b7 ${row.accounts.length} accounts`
                  : row.bankLine || "\u2014"}
              </TableCell>
              <TableCell className="text-right tabular-nums">{row.vouchers}</TableCell>
              <TableCell className="text-right tabular-nums text-navy">
                {formatRupees(row.due)}
              </TableCell>
            </tr>
          )}
        />
      </div>
    </ModuleFrame>
  );
}

function Kpi({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="rounded-lg bg-paper-raised p-4 ring-1 ring-line">
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className={`money-figure mt-1 text-2xl ${accent ? "text-navy" : ""}`}>{value}</p>
    </div>
  );
}

function CountCard({
  label,
  value,
  onClick,
}: {
  label: string;
  value: string;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      className="pressable rounded-lg bg-paper-raised p-4 text-left ring-1 ring-line"
      onClick={onClick}
    >
      <p className="text-xs text-ink-subtle">{label}</p>
      <p className="money-figure mt-1 text-3xl">{value}</p>
    </button>
  );
}
