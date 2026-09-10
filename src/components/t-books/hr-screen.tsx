import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { calcPayroll, formatInr, formatRupees } from "@/lib/t-books/business_rules";
import {
  deleteHrPayroll,
  deleteHrPerson,
  listHrPayroll,
  listHrPeople,
  saveHrPayroll,
  saveHrPerson,
} from "@/lib/t-books/office";
import { invokeErrorMessage } from "@/lib/t-books/platform";
import type { HrPerson, PayrollRow } from "@/lib/t-books/types";
import { cn } from "@/lib/utils";
import { ModuleFrame } from "./module-frame";
import { TableCell, TableHeadCell, VirtualTable } from "./virtual-table";
import { useRegisterUnsaved, useUnsaved } from "./unsaved-guard";

type Tab = "people" | "payroll";

type PersonDraft = {
  id: number | null;
  name: string;
  role: string;
  salary: string;
};

type PayDraft = {
  id: number | null;
  personId: string;
  month: string;
  pfEmployee: string;
  pfCompany: string;
  tds: string;
  totalPaid: string;
};

function blankPerson(): PersonDraft {
  return { id: null, name: "", role: "", salary: "" };
}
function blankPay(): PayDraft {
  return { id: null, personId: "", month: "", pfEmployee: "0", pfCompany: "0", tds: "0", totalPaid: "0" };
}

export default function HrScreen({ onBack }: { onBack: () => void }) {
  const { requestLeave } = useUnsaved();
  const [tab, setTab] = useState<Tab>("people");
  const [people, setPeople] = useState<HrPerson[] | null>(null);
  const [payroll, setPayroll] = useState<PayrollRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editingPerson, setEditingPerson] = useState(false);
  const [editingPay, setEditingPay] = useState(false);
  const [person, setPerson] = useState<PersonDraft>(blankPerson);
  const [savedPerson, setSavedPerson] = useState<PersonDraft>(blankPerson);
  const [pay, setPay] = useState<PayDraft>(blankPay);
  const [savedPay, setSavedPay] = useState<PayDraft>(blankPay);
  const [busy, setBusy] = useState(false);

  const dirty =
    (editingPerson && JSON.stringify(person) !== JSON.stringify(savedPerson)) ||
    (editingPay && JSON.stringify(pay) !== JSON.stringify(savedPay));

  const reload = useCallback(async () => {
    try {
      const [p, r] = await Promise.all([listHrPeople(), listHrPayroll()]);
      setPeople(p);
      setPayroll(r);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
      if (!people) setPeople([]);
      if (!payroll) setPayroll([]);
    }
  }, [people, payroll]);

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const persist = useCallback(async () => {
    if (editingPerson) {
      const salary = Number(person.salary);
      if (person.salary.trim() && !Number.isFinite(salary)) throw new Error("Salary must be a number.");
      const saved = await saveHrPerson({
        id: person.id,
        name: person.name,
        role: person.role,
        salary: Number.isFinite(salary) ? salary : 0,
      });
      const next = {
        id: saved.id ?? null,
        name: saved.name,
        role: saved.role,
        salary: String(saved.salary ?? 0),
      };
      setPerson(next);
      setSavedPerson(next);
    }
    if (editingPay) {
      const n = (raw: string, field: string) => {
        const v = Number(raw);
        if (raw.trim() && !Number.isFinite(v)) throw new Error(`${field} must be a number.`);
        return Number.isFinite(v) ? v : 0;
      };
      const saved = await saveHrPayroll({
        id: pay.id,
        personId: Number(pay.personId),
        personName: "",
        month: pay.month,
        pfEmployee: n(pay.pfEmployee, "PF employee"),
        pfCompany: n(pay.pfCompany, "PF company"),
        pfTotal: 0,
        tds: n(pay.tds, "TDS"),
        totalPaid: n(pay.totalPaid, "Total paid"),
      });
      const next = fromPay(saved);
      setPay(next);
      setSavedPay(next);
    }
    await reload();
  }, [editingPerson, editingPay, person, pay, reload]);

  useRegisterUnsaved(dirty, persist);

  const pfPreview = useMemo(() => {
    const company = Number(pay.pfCompany);
    const employee = Number(pay.pfEmployee);
    const tds = Number(pay.tds);
    return calcPayroll({
      pf_company_rupees: Number.isFinite(company) ? company : 0,
      pf_employee_rupees: Number.isFinite(employee) ? employee : 0,
      tds_rupees: Number.isFinite(tds) ? tds : 0,
    });
  }, [pay]);

  async function onSave() {
    setBusy(true);
    try {
      await persist();
      setEditingPerson(false);
      setEditingPay(false);
      setError(null);
    } catch (err) {
      setError(invokeErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  if (editingPerson) {
    return (
      <ModuleFrame
        title={person.id ? "Edit person" : "New person"}
        onBack={() => requestLeave(() => setEditingPerson(false))}
        error={error}
        actions={
          <Button onClick={() => void onSave()} disabled={busy}>
            {busy ? "Saving…" : "Save"}
          </Button>
        }
      >
        <div className="grid gap-4 rounded-lg bg-paper-raised p-5 ring-1 ring-line sm:grid-cols-2">
          <div className="sm:col-span-2">
            <Label htmlFor="hr-name">Name</Label>
            <Input
              id="hr-name"
              className="mt-1.5"
              value={person.name}
              onChange={(e) => setPerson({ ...person, name: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="hr-role">Role</Label>
            <Input
              id="hr-role"
              className="mt-1.5"
              value={person.role}
              onChange={(e) => setPerson({ ...person, role: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="hr-salary">Salary</Label>
            <Input
              id="hr-salary"
              className="mt-1.5"
              inputMode="decimal"
              value={person.salary}
              onChange={(e) => setPerson({ ...person, salary: e.target.value })}
            />
          </div>
        </div>
      </ModuleFrame>
    );
  }

  if (editingPay) {
    return (
      <ModuleFrame
        title={pay.id ? "Edit payroll" : "New payroll"}
        hint="PF total is employee + company (calcPayroll). One row per person per month."
        onBack={() => requestLeave(() => setEditingPay(false))}
        error={error}
        actions={
          <Button onClick={() => void onSave()} disabled={busy}>
            {busy ? "Saving…" : "Save"}
          </Button>
        }
      >
        <div className="grid gap-4 rounded-lg bg-paper-raised p-5 ring-1 ring-line sm:grid-cols-2">
          <div>
            <Label htmlFor="pay-person">Person</Label>
            <select
              id="pay-person"
              className="mt-1.5 h-11 w-full rounded-md bg-paper-raised px-3 text-base text-ink shadow-[0_0_0_1px_var(--color-line)] focus-visible:outline-none focus-visible:shadow-[0_0_0_2px_var(--color-navy)]"
              value={pay.personId}
              onChange={(e) => setPay({ ...pay, personId: e.target.value })}
            >
              <option value="">Select</option>
              {(people ?? []).map((p) => (
                <option key={p.id ?? p.name} value={p.id ?? ""}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <Label htmlFor="pay-month">Month</Label>
            <Input
              id="pay-month"
              className="mt-1.5"
              type="month"
              value={pay.month}
              onChange={(e) => setPay({ ...pay, month: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="pay-pfe">PF employee</Label>
            <Input
              id="pay-pfe"
              className="mt-1.5"
              inputMode="decimal"
              value={pay.pfEmployee}
              onChange={(e) => setPay({ ...pay, pfEmployee: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="pay-pfc">PF company</Label>
            <Input
              id="pay-pfc"
              className="mt-1.5"
              inputMode="decimal"
              value={pay.pfCompany}
              onChange={(e) => setPay({ ...pay, pfCompany: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="pay-tds">TDS</Label>
            <Input
              id="pay-tds"
              className="mt-1.5"
              inputMode="decimal"
              value={pay.tds}
              onChange={(e) => setPay({ ...pay, tds: e.target.value })}
            />
          </div>
          <div>
            <Label htmlFor="pay-total">Total paid</Label>
            <Input
              id="pay-total"
              className="mt-1.5"
              inputMode="decimal"
              value={pay.totalPaid}
              onChange={(e) => setPay({ ...pay, totalPaid: e.target.value })}
            />
          </div>
          <div className="rounded-lg bg-navy-soft px-3 py-2 sm:col-span-2">
            <p className="text-xs uppercase tracking-wide text-navy">PF total</p>
            <p className="money-figure mt-1 text-lg text-navy">
              {formatInr(pfPreview.pf_total_paise)}
            </p>
          </div>
        </div>
      </ModuleFrame>
    );
  }

  return (
    <ModuleFrame
      title="HR"
      hint="People and payroll stay on this PC. Refresh from Google does not change these rows."
      onBack={() => requestLeave(onBack)}
      error={error}
      actions={
        tab === "people" ? (
          <Button
            onClick={() => {
              const next = blankPerson();
              setPerson(next);
              setSavedPerson(next);
              setEditingPerson(true);
            }}
          >
            New person
          </Button>
        ) : (
          <Button
            onClick={() => {
              const next = blankPay();
              setPay(next);
              setSavedPay(next);
              setEditingPay(true);
            }}
          >
            New payroll
          </Button>
        )
      }
    >
      <div className="mb-4 flex gap-2">
        {(["people", "payroll"] as const).map((id) => (
          <button
            key={id}
            type="button"
            className={cn(
              "pressable h-11 rounded-md px-4 text-sm capitalize",
              tab === id ? "bg-navy text-paper-raised" : "bg-paper-sunken text-ink-muted",
            )}
            onClick={() => setTab(id)}
          >
            {id}
          </button>
        ))}
      </div>
      <div className="overflow-hidden rounded-lg bg-paper-raised ring-1 ring-line">
        {tab === "people" ? (
          <VirtualTable
            rows={people ?? []}
            rowKey={(row, i) => row.id ?? i}
            empty={
              <p className="text-sm text-ink-muted">
                {people === null ? "Opening people on this PC…" : "No people on this PC yet."}
              </p>
            }
            header={
              <tr>
                <TableHeadCell>Name</TableHeadCell>
                <TableHeadCell>Role</TableHeadCell>
                <TableHeadCell className="text-right">Salary</TableHeadCell>
                <TableHeadCell />
              </tr>
            }
            renderRow={(row) => (
              <tr className="table-row">
                <TableCell>{row.name}</TableCell>
                <TableCell className="text-ink-muted">{row.role || "—"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.salary)}</TableCell>
                <TableCell>
                  <div className="flex justify-end gap-2">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        const next = {
                          id: row.id ?? null,
                          name: row.name,
                          role: row.role,
                          salary: String(row.salary ?? 0),
                        };
                        setPerson(next);
                        setSavedPerson(next);
                        setEditingPerson(true);
                      }}
                    >
                      Edit
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy || !row.id}
                      onClick={() => {
                        if (!row.id) return;
                        void (async () => {
                          setBusy(true);
                          try {
                            await deleteHrPerson(row.id as number);
                            await reload();
                          } catch (err) {
                            setError(invokeErrorMessage(err));
                          } finally {
                            setBusy(false);
                          }
                        })();
                      }}
                    >
                      Delete
                    </Button>
                  </div>
                </TableCell>
              </tr>
            )}
          />
        ) : (
          <VirtualTable
            rows={payroll ?? []}
            rowKey={(row, i) => row.id ?? i}
            empty={
              <p className="text-sm text-ink-muted">
                {payroll === null ? "Opening payroll on this PC…" : "No payroll rows on this PC yet."}
              </p>
            }
            header={
              <tr>
                <TableHeadCell>Person</TableHeadCell>
                <TableHeadCell>Month</TableHeadCell>
                <TableHeadCell className="text-right">PF total</TableHeadCell>
                <TableHeadCell className="text-right">TDS</TableHeadCell>
                <TableHeadCell className="text-right">Paid</TableHeadCell>
                <TableHeadCell />
              </tr>
            }
            renderRow={(row) => (
              <tr className="table-row">
                <TableCell>{row.personName}</TableCell>
                <TableCell className="text-ink-muted">{row.month}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.pfTotal)}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.tds)}</TableCell>
                <TableCell className="text-right tabular-nums">{formatRupees(row.totalPaid)}</TableCell>
                <TableCell>
                  <div className="flex justify-end gap-2">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        const next = fromPay(row);
                        setPay(next);
                        setSavedPay(next);
                        setEditingPay(true);
                      }}
                    >
                      Edit
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy || !row.id}
                      onClick={() => {
                        if (!row.id) return;
                        void (async () => {
                          setBusy(true);
                          try {
                            await deleteHrPayroll(row.id as number);
                            await reload();
                          } catch (err) {
                            setError(invokeErrorMessage(err));
                          } finally {
                            setBusy(false);
                          }
                        })();
                      }}
                    >
                      Delete
                    </Button>
                  </div>
                </TableCell>
              </tr>
            )}
          />
        )}
      </div>
    </ModuleFrame>
  );
}

function fromPay(row: PayrollRow): PayDraft {
  return {
    id: row.id ?? null,
    personId: String(row.personId ?? ""),
    month: row.month,
    pfEmployee: String(row.pfEmployee ?? 0),
    pfCompany: String(row.pfCompany ?? 0),
    tds: String(row.tds ?? 0),
    totalPaid: String(row.totalPaid ?? 0),
  };
}
