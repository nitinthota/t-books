import {
  allocMethodForPurchase,
  calcPayroll,
  calcPo,
  calcPoItem,
  calcSalarySlip,
  classifyPurchasePayment,
  keepPostedPayNumber,
  nextPaymentNumber,
  nextPurchaseNumber,
  nextSalaryNumber,
  normalizeAllocMethod,
  normalizePayKind,
  paiseToRupees,
  purchasePayStatus,
  rupeesToPaise,
  salaryPeriodTaken,
  taxInvoiceMissing,
} from "./business_rules";
import { all, exec, lastInsertId, withTransaction } from "./db";
import { invokeCommand, isTauriRuntime } from "./platform";
import { getTrialLocal, listFyLocal } from "./trial";
import type {
  CasOutcome,
  DocumentRow,
  HrPerson,
  InventoryRow,
  LogisticsRow,
  PayrollRow,
  PoItemIn,
  PoPreview,
  PurchasePayment,
  PurchasePaymentSave,
  PurchasePo,
  PurchasePoSave,
  SalesPo,
  SalesPoSave,
  SearchHit,
  TrialBalance,
  VendorRef,
} from "./types";

type SqlRow = Record<string, string | number | null | Uint8Array>;

function text(row: SqlRow, key: string): string {
  const value = row[key];
  return value == null || value instanceof Uint8Array ? "" : String(value);
}

function num(row: SqlRow, key: string): number {
  const value = row[key];
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const n = Number(value);
    return Number.isFinite(n) ? n : 0;
  }
  return 0;
}

function trim(value: string | null | undefined): string {
  return (value ?? "").trim();
}

function finite(value: number | null | undefined): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function requireText(value: string, field: string): string {
  const t = trim(value);
  if (!t) throw new Error(`${field} is required.`);
  return t;
}

function validMonth(value: string): boolean {
  return /^\d{4}-\d{2}$/.test(value);
}

function isUnique(err: unknown): boolean {
  const message = err instanceof Error ? err.message : String(err);
  return /UNIQUE constraint/i.test(message);
}

function likePattern(query: string): string {
  return `%${query.replace(/\\/g, "\\\\").replace(/%/g, "\\%").replace(/_/g, "\\_")}%`;
}

function touchProject(project: string): void {
  if (!project) return;
  exec(
    `INSERT INTO projects (project, updated_at) VALUES (?, datetime('now'))
     ON CONFLICT(project) DO UPDATE SET updated_at = excluded.updated_at`,
    [project],
  );
}

function touchVendor(vendor: string): void {
  if (!vendor) return;
  exec(
    `INSERT INTO vendors (vendor, updated_at) VALUES (?, datetime('now'))
     ON CONFLICT(vendor) DO NOTHING`,
    [vendor],
  );
}

export function previewPo(items: PoItemIn[]): PoPreview {
  const summary = calcPo({
    payment_term_value: 0,
    payment_term_unit: "days",
    amount_received_rupees: 0,
    items: items.map((item) => ({
      item_name: item.itemName,
      description: item.description,
      qty: finite(item.qty),
      unit_rate_rupees: finite(item.rate),
      gst_pct: finite(item.gstPct),
    })),
  });
  return {
    items: items.map((item, i) => {
      const calc =
        summary.items[i] ??
        calcPoItem({
          item_name: item.itemName,
          description: item.description,
          qty: finite(item.qty),
          unit_rate_rupees: finite(item.rate),
          gst_pct: finite(item.gstPct),
        });
      return {
        id: item.id,
        itemName: item.itemName || calc.item_name,
        description: calc.description,
        qty: calc.qty,
        rate: item.rate,
        gstPct: calc.gst_pct,
        amount: paiseToRupees(calc.total_cost_paise),
      };
    }),
    subtotal: paiseToRupees(summary.subtotal_paise),
    gst: paiseToRupees(summary.gst_paise),
    grandTotal: paiseToRupees(summary.grand_total_paise),
  };
}

function loadItems(table: string, poId: number): PoItemIn[] {
  return all<SqlRow>(
    `SELECT id, COALESCE(item_name,'') AS item_name, description, qty, rate, gst_pct, amount FROM ${table} WHERE po_id = ? ORDER BY id`,
    [poId],
  ).map((row) => ({
    id: num(row, "id"),
    itemName: text(row, "item_name"),
    description: text(row, "description"),
    qty: num(row, "qty"),
    rate: num(row, "rate"),
    gstPct: num(row, "gst_pct"),
    amount: num(row, "amount"),
  }));
}

function replaceItems(table: string, poId: number, items: PoItemIn[]): void {
  exec(`DELETE FROM ${table} WHERE po_id = ?`, [poId]);
  for (const item of items) {
    const calc = calcPoItem({
      item_name: item.itemName,
      description: item.description,
      qty: finite(item.qty),
      unit_rate_rupees: finite(item.rate),
      gst_pct: finite(item.gstPct),
    });
    exec(
      `INSERT INTO ${table} (po_id, item_name, description, qty, rate, gst_pct, amount) VALUES (?, ?, ?, ?, ?, ?, ?)`,
      [
        poId,
        (item.itemName ?? "").trim() || calc.item_name,
        calc.description,
        calc.qty,
        finite(item.rate),
        calc.gst_pct,
        paiseToRupees(calc.total_cost_paise),
      ],
    );
  }
}

function poTaken(table: string, project: string, poNumber: string, skipId?: number | null): boolean {
  const rows = all<SqlRow>(
    `SELECT id FROM ${table} WHERE project = ? COLLATE NOCASE AND po_number = ? COLLATE NOCASE LIMIT 1`,
    [project, poNumber],
  );
  const found = rows[0] ? num(rows[0], "id") : 0;
  if (!found) return false;
  return skipId == null || found !== skipId;
}

function mapSales(row: SqlRow, withItems: boolean): SalesPo {
  const id = num(row, "id");
  return {
    id,
    project: text(row, "project"),
    poNumber: text(row, "po_number"),
    client: text(row, "client"),
    gst: text(row, "gst"),
    totalValue: num(row, "total_value"),
    createdAt: text(row, "created_at"),
    updatedAt: text(row, "updated_at"),
    isDirty: num(row, "is_dirty") !== 0,
    items: withItems ? loadItems("sales_po_items", id) : [],
  };
}

function listSalesPoLocal(): SalesPo[] {
  return all<SqlRow>(
    `SELECT id, project, po_number, client, gst, total_value, created_at, updated_at,
            COALESCE(is_dirty,0) AS is_dirty
     FROM sales_po ORDER BY updated_at DESC, id DESC`,
  ).map((row) => mapSales(row, false));
}

function getSalesPoLocal(id: number): SalesPo {
  const rows = all<SqlRow>(
    `SELECT id, project, po_number, client, gst, total_value, created_at, updated_at,
            COALESCE(is_dirty,0) AS is_dirty
     FROM sales_po WHERE id = ?`,
    [id],
  );
  if (!rows[0]) throw new Error("That sales PO was not found on this PC.");
  return mapSales(rows[0], true);
}

function saveSalesPoLocal(payload: SalesPoSave): SalesPo {
  const project = requireText(payload.project, "Project");
  const poNumber = requireText(payload.poNumber, "PO number");
  if (poTaken("sales_po", project, poNumber, payload.id)) {
    throw new Error("A sales PO with this number already exists on this project.");
  }
  const preview = previewPo(payload.items ?? []);
  const total = preview.grandTotal;
  const client = trim(payload.client);
  const gst = trim(payload.gst);
  let id = payload.id ?? 0;
  try {
    withTransaction(() => {
      touchProject(project);
      if (id) {
        exec(
          `UPDATE sales_po SET project = ?, po_number = ?, client = ?, gst = ?, total_value = ?,
           is_dirty = 1, updated_at = datetime('now') WHERE id = ?`,
          [project, poNumber, client, gst, total, id],
        );
      } else {
        exec(
          `INSERT INTO sales_po (project, po_number, client, gst, total_value, is_dirty, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 1, datetime('now'), datetime('now'))`,
          [project, poNumber, client, gst, total],
        );
        id = lastInsertId();
      }
      replaceItems("sales_po_items", id, payload.items ?? []);
    });
  } catch (err) {
    if (isUnique(err)) {
      throw new Error("A sales PO with this number already exists on this project.");
    }
    throw err;
  }
  return getSalesPoLocal(id);
}

function deleteSalesPoLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM sales_po WHERE id = ?", [id]);
  });
}

function paidForPo(poNumber: string): number {
  const rows = all<SqlRow>(
    "SELECT COALESCE(SUM(amount_rupees), 0) AS paid FROM purchase_payments WHERE po_number = ? COLLATE NOCASE",
    [poNumber],
  );
  return num(rows[0] ?? {}, "paid");
}

function decoratePurchase(row: PurchasePo): PurchasePo {
  const paid = paidForPo(row.poNumber);
  return {
    ...row,
    paidRupees: paid,
    payStatus: purchasePayStatus(rupeesToPaise(row.totalValue), rupeesToPaise(paid)),
  };
}

function mapPurchase(row: SqlRow, withItems: boolean): PurchasePo {
  const id = num(row, "id");
  const rawType = text(row, "type").toLowerCase() === "simple" ? "simple" : "contract";
  return decoratePurchase({
    id,
    project: text(row, "project"),
    vendor: text(row, "vendor"),
    poNumber: text(row, "po_number"),
    type: rawType,
    totalValue: num(row, "total_value"),
    createdAt: text(row, "created_at"),
    updatedAt: text(row, "updated_at"),
    goodsReceived: num(row, "goods_received") !== 0,
    taxInvoiceNo: text(row, "tax_invoice_no"),
    taxInvoiceDate: text(row, "tax_invoice_date"),
    isDirty: num(row, "is_dirty") !== 0,
    paidRupees: 0,
    payStatus: "",
    items: withItems ? loadItems("purchase_po_items", id) : [],
  });
}

function listPurchasePoLocal(): PurchasePo[] {
  return all<SqlRow>(
    `SELECT id, project, vendor, po_number, type, total_value, created_at, updated_at,
            COALESCE(goods_received,0) AS goods_received, COALESCE(tax_invoice_no,'') AS tax_invoice_no,
            COALESCE(tax_invoice_date,'') AS tax_invoice_date, COALESCE(is_dirty,0) AS is_dirty
     FROM purchase_po ORDER BY updated_at DESC, id DESC`,
  ).map((row) => mapPurchase(row, false));
}

function getPurchasePoLocal(id: number): PurchasePo {
  const rows = all<SqlRow>(
    `SELECT id, project, vendor, po_number, type, total_value, created_at, updated_at,
            COALESCE(goods_received,0) AS goods_received, COALESCE(tax_invoice_no,'') AS tax_invoice_no,
            COALESCE(tax_invoice_date,'') AS tax_invoice_date, COALESCE(is_dirty,0) AS is_dirty
     FROM purchase_po WHERE id = ?`,
    [id],
  );
  if (!rows[0]) throw new Error("That purchase PO was not found on this PC.");
  return mapPurchase(rows[0], true);
}

function savePurchasePoLocal(payload: PurchasePoSave): PurchasePo {
  const project = requireText(payload.project, "Project");
  const vendor = requireText(payload.vendor, "Vendor");
  const poType = payload.type === "simple" ? "simple" : "contract";
  const existingRows = payload.id
    ? all<SqlRow>("SELECT po_number FROM purchase_po WHERE id = ?", [payload.id])
    : [];
  const existingNumber = existingRows[0] ? text(existingRows[0], "po_number") : "";
  const requested = trim(payload.poNumber);
  const listed = all<SqlRow>("SELECT po_number FROM purchase_po WHERE TRIM(po_number) != ''").map((r) =>
    text(r, "po_number"),
  );
  const poNumber = existingNumber
    ? keepPostedPayNumber(existingNumber, requested)
    : requested || nextPurchaseNumber(listed);
  const taken = all<SqlRow>(
    "SELECT id FROM purchase_po WHERE po_number = ? COLLATE NOCASE LIMIT 1",
    [poNumber],
  );
  if (taken[0] && num(taken[0], "id") !== (payload.id ?? 0)) {
    throw new Error("A purchase bill with this number already exists.");
  }
  if (poTaken("purchase_po", project, poNumber, payload.id)) {
    throw new Error("A purchase PO with this number already exists on this project.");
  }
  const items = poType === "simple" ? [] : (payload.items ?? []);
  const total = poType === "simple" ? finite(payload.totalValue) : previewPo(items).grandTotal;
  const goods = payload.goodsReceived ? 1 : 0;
  const taxNo = trim(payload.taxInvoiceNo ?? "");
  const taxDate = trim(payload.taxInvoiceDate ?? "");
  let id = payload.id ?? 0;
  try {
    withTransaction(() => {
      touchProject(project);
      touchVendor(vendor);
      if (id) {
        exec(
          `UPDATE purchase_po SET project = ?, vendor = ?, po_number = ?, type = ?, total_value = ?,
           goods_received = ?, tax_invoice_no = ?, tax_invoice_date = ?, is_dirty = 1,
           updated_at = datetime('now') WHERE id = ?`,
          [project, vendor, poNumber, poType, total, goods, taxNo, taxDate, id],
        );
      } else {
        exec(
          `INSERT INTO purchase_po (project, vendor, po_number, type, total_value, goods_received,
           tax_invoice_no, tax_invoice_date, is_dirty, hive_rev, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, 0, datetime('now'), datetime('now'))`,
          [project, vendor, poNumber, poType, total, goods, taxNo, taxDate],
        );
        id = lastInsertId();
      }
      replaceItems("purchase_po_items", id, items);
    });
  } catch (err) {
    if (isUnique(err)) {
      throw new Error("A purchase bill with this number already exists.");
    }
    throw err;
  }
  return getPurchasePoLocal(id);
}

function deletePurchasePoLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM purchase_po WHERE id = ?", [id]);
  });
}

function mapPayment(row: SqlRow): PurchasePayment {
  return {
    id: num(row, "id"),
    payNumber: text(row, "pay_number"),
    poNumber: text(row, "po_number"),
    vendor: text(row, "vendor"),
    project: text(row, "project"),
    amountRupees: num(row, "amount_rupees"),
    allocMethod: text(row, "alloc_method"),
    payClass: text(row, "pay_class"),
    missingTaxInvoice: num(row, "missing_tax_invoice") !== 0,
    payDate: text(row, "pay_date"),
    remarks: text(row, "remarks"),
    isDirty: num(row, "is_dirty") !== 0,
  };
}

function listPurchasePaymentsLocal(poNumber?: string | null): PurchasePayment[] {
  const po = trim(poNumber ?? "");
  const sql = po
    ? `SELECT id, pay_number, po_number, vendor, project, amount_rupees, alloc_method, pay_class,
              COALESCE(missing_tax_invoice,0) AS missing_tax_invoice, COALESCE(pay_date,'') AS pay_date,
              COALESCE(remarks,'') AS remarks, COALESCE(is_dirty,0) AS is_dirty
       FROM purchase_payments WHERE po_number = ? COLLATE NOCASE ORDER BY pay_number COLLATE NOCASE, id`
    : `SELECT id, pay_number, po_number, vendor, project, amount_rupees, alloc_method, pay_class,
              COALESCE(missing_tax_invoice,0) AS missing_tax_invoice, COALESCE(pay_date,'') AS pay_date,
              COALESCE(remarks,'') AS remarks, COALESCE(is_dirty,0) AS is_dirty
       FROM purchase_payments ORDER BY pay_number COLLATE NOCASE, id`;
  return all<SqlRow>(sql, po ? [po] : []).map(mapPayment);
}

function savePurchasePaymentLocal(payload: PurchasePaymentSave): PurchasePayment {
  const poNumber = requireText(payload.poNumber, "Purchase number");
  const amount = finite(payload.amountRupees);
  if (amount <= 0) throw new Error("Payment amount must be greater than zero.");
  const bills = all<SqlRow>(
    `SELECT vendor, project, total_value, COALESCE(goods_received,0) AS goods_received,
            COALESCE(tax_invoice_no,'') AS tax_invoice_no, COALESCE(tax_invoice_date,'') AS tax_invoice_date
     FROM purchase_po WHERE po_number = ? COLLATE NOCASE LIMIT 1`,
    [poNumber],
  );
  if (!bills[0]) throw new Error("That purchase bill was not found on this PC.");
  const bill = bills[0];
  const vendor = trim(payload.vendor) || text(bill, "vendor");
  const project = text(bill, "project");
  const existing = payload.id
    ? all<SqlRow>("SELECT pay_number FROM purchase_payments WHERE id = ?", [payload.id])
    : [];
  const existingNumber = existing[0] ? text(existing[0], "pay_number") : "";
  const listed = all<SqlRow>("SELECT pay_number FROM purchase_payments WHERE TRIM(pay_number) != ''").map((r) =>
    text(r, "pay_number"),
  );
  const requested = trim(payload.payNumber);
  const payNumber = existingNumber
    ? keepPostedPayNumber(existingNumber, requested)
    : requested || nextPaymentNumber(listed, poNumber);
  const skipId = payload.id ?? -1;
  const beforeRows = all<SqlRow>(
    "SELECT COALESCE(SUM(amount_rupees),0) AS paid FROM purchase_payments WHERE po_number = ? COLLATE NOCASE AND id != ?",
    [poNumber, skipId],
  );
  const paidBefore = num(beforeRows[0] ?? {}, "paid");
  const goods = num(bill, "goods_received") !== 0;
  const missingTax = taxInvoiceMissing(text(bill, "tax_invoice_no"), text(bill, "tax_invoice_date"));
  const classified = classifyPurchasePayment({
    goodsReceived: goods,
    taxInvoiceMissing: missingTax,
    grandPaise: rupeesToPaise(num(bill, "total_value")),
    paidBeforePaise: rupeesToPaise(paidBefore),
    thisPaise: rupeesToPaise(amount),
  });
  const alloc = trim(payload.allocMethod)
    ? normalizeAllocMethod(payload.allocMethod)
    : allocMethodForPurchase(goods);
  let id = payload.id ?? 0;
  withTransaction(() => {
    if (id) {
      exec(
        `UPDATE purchase_payments SET pay_number = ?, po_number = ?, vendor = ?, project = ?,
         amount_rupees = ?, alloc_method = ?, pay_class = ?, missing_tax_invoice = ?,
         pay_date = ?, remarks = ?, is_dirty = 1, updated_at = datetime('now') WHERE id = ?`,
        [payNumber, poNumber, vendor, project, amount, alloc, classified.payClass, classified.missingTaxInvoice ? 1 : 0, trim(payload.payDate), trim(payload.remarks), id],
      );
    } else {
      exec(
        `INSERT INTO purchase_payments (pay_number, po_number, vendor, project, amount_rupees,
         alloc_method, pay_class, missing_tax_invoice, pay_date, remarks, is_dirty, hive_rev,
         created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0, datetime('now'), datetime('now'))`,
        [payNumber, poNumber, vendor, project, amount, alloc, classified.payClass, classified.missingTaxInvoice ? 1 : 0, trim(payload.payDate), trim(payload.remarks)],
      );
      id = lastInsertId();
    }
  });
  const rows = all<SqlRow>(
    `SELECT id, pay_number, po_number, vendor, project, amount_rupees, alloc_method, pay_class,
            COALESCE(missing_tax_invoice,0) AS missing_tax_invoice, COALESCE(pay_date,'') AS pay_date,
            COALESCE(remarks,'') AS remarks, COALESCE(is_dirty,0) AS is_dirty
     FROM purchase_payments WHERE id = ?`,
    [id],
  );
  if (!rows[0]) throw new Error("That payment was not found on this PC.");
  return mapPayment(rows[0]);
}

function deletePurchasePaymentLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM purchase_payments WHERE id = ?", [id]);
  });
}

function mapPerson(row: SqlRow): HrPerson {
  return {
    id: num(row, "id"),
    name: text(row, "name"),
    role: text(row, "role"),
    salary: num(row, "salary"),
    active: text(row, "active") || "Yes",
  };
}

function listHrPeopleLocal(): HrPerson[] {
  return all<SqlRow>(
    "SELECT id, name, role, salary, COALESCE(active,'Yes') AS active FROM hr_people ORDER BY name COLLATE NOCASE, id",
  ).map(mapPerson);
}

function saveHrPersonLocal(payload: HrPerson): HrPerson {
  const name = requireText(payload.name, "Name");
  const role = trim(payload.role);
  const salary = finite(payload.salary);
  const active = ["no", "n", "false", "0"].includes(trim(payload.active ?? "Yes").toLowerCase())
    ? "No"
    : "Yes";
  if (active === "Yes" && salary <= 0) {
    throw new Error("Salary is required when a person is Active.");
  }
  let id = payload.id ?? 0;
  withTransaction(() => {
    if (id) {
      exec("UPDATE hr_people SET name = ?, role = ?, salary = ?, active = ? WHERE id = ?", [
        name, role, salary, active, id,
      ]);
    } else {
      exec("INSERT INTO hr_people (name, role, salary, active) VALUES (?, ?, ?, ?)", [
        name, role, salary, active,
      ]);
      id = lastInsertId();
    }
  });
  const rows = all<SqlRow>(
    "SELECT id, name, role, salary, COALESCE(active,'Yes') AS active FROM hr_people WHERE id = ?",
    [id],
  );
  if (!rows[0]) throw new Error("That person was not found on this PC.");
  return mapPerson(rows[0]);
}

function deleteHrPersonLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM hr_people WHERE id = ?", [id]);
  });
}

function mapPayroll(row: SqlRow): PayrollRow {
  const pfEmployee = num(row, "pf_employee");
  const pfCompany = num(row, "pf_company");
  const tds = num(row, "tds");
  const payKind = text(row, "pay_kind") || "salary";
  const salaryRupees = num(row, "salary_rupees") || num(row, "salary");
  const recoveryRupees = num(row, "recovery_rupees");
  const slip = calcSalarySlip({
    kind: payKind,
    salary_paise: rupeesToPaise(salaryRupees),
    pf_employee_paise: rupeesToPaise(pfEmployee),
    pf_company_paise: rupeesToPaise(pfCompany),
    tds_paise: rupeesToPaise(tds),
    recovery_paise: rupeesToPaise(recoveryRupees),
    amount_paise: rupeesToPaise(num(row, "total_paid")),
  });
  const calc = calcPayroll({
    pf_company_rupees: pfCompany,
    pf_employee_rupees: pfEmployee,
    tds_rupees: tds,
  });
  return {
    id: num(row, "id"),
    personId: num(row, "person_id"),
    personName: text(row, "name") || text(row, "person_name"),
    month: text(row, "month"),
    pfEmployee,
    pfCompany,
    pfTotal: paiseToRupees(calc.pf_total_paise),
    tds,
    totalPaid: paiseToRupees(slip.net_paise),
    salaryNumber: text(row, "salary_number"),
    payKind,
    salaryRupees,
    recoveryRupees: paiseToRupees(slip.recovery_paise),
    netRupees: paiseToRupees(slip.net_paise),
    isDirty: num(row, "is_dirty") !== 0,
  };
}

const PAYROLL_SELECT = `SELECT p.id, p.person_id, h.name, p.month, p.pf_employee, p.pf_company, p.tds, p.total_paid,
        h.salary, COALESCE(p.salary_number,'') AS salary_number, COALESCE(p.pay_kind,'salary') AS pay_kind,
        COALESCE(p.salary_rupees,0) AS salary_rupees, COALESCE(p.recovery_rupees,0) AS recovery_rupees,
        COALESCE(p.is_dirty,0) AS is_dirty
 FROM hr_payroll p JOIN hr_people h ON h.id = p.person_id`;

function listHrPayrollLocal(): PayrollRow[] {
  return all<SqlRow>(`${PAYROLL_SELECT} ORDER BY p.month DESC, h.name COLLATE NOCASE`).map(mapPayroll);
}

function parseMonthParts(month: string): { month: number; year: number } | null {
  if (!validMonth(month)) return null;
  const year = Number(month.slice(0, 4));
  const m = Number(month.slice(5, 7));
  if (!Number.isFinite(year) || !Number.isFinite(m)) return null;
  return { month: m, year };
}

function saveHrPayrollLocal(payload: PayrollRow): PayrollRow {
  if (!payload.personId) throw new Error("Person is required.");
  const month = trim(payload.month);
  if (!validMonth(month)) throw new Error("Month must be YYYY-MM.");
  const people = all<SqlRow>(
    "SELECT id, name, salary, COALESCE(active,'Yes') AS active FROM hr_people WHERE id = ?",
    [payload.personId],
  );
  if (!people[0]) throw new Error("That person was not found on this PC.");
  const personName = text(people[0], "name");
  const personSalary = num(people[0], "salary");
  const active = text(people[0], "active") || "Yes";
  const kind = normalizePayKind(payload.payKind);
  const parts = parseMonthParts(month);
  if (!parts) throw new Error("Month must be YYYY-MM.");
  if (kind === "salary") {
    const existing = all<SqlRow>(
      "SELECT id, person_id, pay_kind, month FROM hr_payroll WHERE pay_kind IS NULL OR pay_kind != 'advance'",
    ).map((row) => {
      const parsed = parseMonthParts(text(row, "month"));
      return {
        id: String(num(row, "id")),
        employee_id: String(num(row, "person_id")),
        employee_name: "",
        pay_kind: text(row, "pay_kind") || "salary",
        period_month: parsed?.month ?? 0,
        period_year: parsed?.year ?? 0,
      };
    });
    if (
      salaryPeriodTaken(existing, {
        id: payload.id ? String(payload.id) : undefined,
        employee_id: String(payload.personId),
        employee_name: personName,
        period_month: parts.month,
        period_year: parts.year,
      })
    ) {
      throw new Error("Payroll for that person already exists in this month.");
    }
  }
  const salaryRupees = finite(payload.salaryRupees) > 0 ? finite(payload.salaryRupees) : personSalary;
  if (kind === "salary" && active === "Yes" && salaryRupees <= 0) {
    throw new Error("Salary is required when a person is Active.");
  }
  const pfEmployee = finite(payload.pfEmployee);
  const pfCompany = finite(payload.pfCompany);
  const tds = finite(payload.tds);
  const recovery = finite(payload.recoveryRupees);
  const slip = calcSalarySlip({
    kind,
    salary_paise: rupeesToPaise(salaryRupees),
    pf_employee_paise: rupeesToPaise(pfEmployee),
    pf_company_paise: rupeesToPaise(pfCompany),
    tds_paise: rupeesToPaise(tds),
    recovery_paise: rupeesToPaise(recovery),
    amount_paise: rupeesToPaise(finite(payload.totalPaid)),
  });
  const totalPaid = paiseToRupees(slip.net_paise);
  const net = paiseToRupees(slip.net_paise);
  let existingNumber = "";
  if (payload.id) {
    const rows = all<SqlRow>("SELECT COALESCE(salary_number,'') AS salary_number FROM hr_payroll WHERE id = ?", [
      payload.id,
    ]);
    existingNumber = rows[0] ? text(rows[0], "salary_number") : "";
  }
  const requested = trim(payload.salaryNumber);
  const listed = all<SqlRow>(
    "SELECT salary_number FROM hr_payroll WHERE TRIM(COALESCE(salary_number,'')) != ''",
  ).map((row) => text(row, "salary_number"));
  const salaryNumber = existingNumber
    ? keepPostedPayNumber(existingNumber, requested)
    : requested || nextSalaryNumber(listed);
  let id = payload.id ?? 0;
  withTransaction(() => {
    if (id) {
      exec(
        `UPDATE hr_payroll SET person_id = ?, month = ?, pf_employee = ?, pf_company = ?, tds = ?, total_paid = ?,
         salary_number = ?, pay_kind = ?, salary_rupees = ?, recovery_rupees = ?, net_rupees = ?, is_dirty = 1
         WHERE id = ?`,
        [
          payload.personId,
          month,
          pfEmployee,
          pfCompany,
          tds,
          totalPaid,
          salaryNumber,
          kind,
          salaryRupees,
          paiseToRupees(slip.recovery_paise),
          net,
          id,
        ],
      );
    } else {
      exec(
        `INSERT INTO hr_payroll (person_id, month, pf_employee, pf_company, tds, total_paid,
         salary_number, pay_kind, salary_rupees, recovery_rupees, net_rupees, is_dirty, hive_rev)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0)`,
        [
          payload.personId,
          month,
          pfEmployee,
          pfCompany,
          tds,
          totalPaid,
          salaryNumber,
          kind,
          salaryRupees,
          paiseToRupees(slip.recovery_paise),
          net,
        ],
      );
      id = lastInsertId();
    }
  });
  const rows = all<SqlRow>(`${PAYROLL_SELECT} WHERE p.id = ?`, [id]);
  if (!rows[0]) throw new Error("That payroll row was not found on this PC.");
  return mapPayroll(rows[0]);
}

function deleteHrPayrollLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM hr_payroll WHERE id = ?", [id]);
  });
}

function mapInventory(row: SqlRow): InventoryRow {
  return {
    id: num(row, "id"),
    itemName: text(row, "item_name"),
    type: text(row, "type"),
    size: text(row, "size"),
    quantity: num(row, "quantity"),
    cost: num(row, "cost"),
    project: text(row, "project"),
  };
}

function listInventoryLocal(query?: string | null): InventoryRow[] {
  const q = trim(query ?? "");
  if (!q) {
    return all<SqlRow>(
      "SELECT id, item_name, type, size, quantity, cost, project FROM inventory ORDER BY item_name COLLATE NOCASE, id",
    ).map(mapInventory);
  }
  const like = likePattern(q);
  return all<SqlRow>(
    `SELECT id, item_name, type, size, quantity, cost, project FROM inventory
     WHERE item_name LIKE ? ESCAPE '\\' OR type LIKE ? ESCAPE '\\'
     ORDER BY item_name COLLATE NOCASE, id`,
    [like, like],
  ).map(mapInventory);
}

function saveInventoryLocal(payload: InventoryRow): InventoryRow {
  const itemName = requireText(payload.itemName, "Item name");
  const itemType = trim(payload.type);
  const size = trim(payload.size);
  const project = trim(payload.project);
  const quantity = finite(payload.quantity);
  const cost = finite(payload.cost);
  let id = payload.id ?? 0;
  withTransaction(() => {
    touchProject(project);
    if (id) {
      exec(
        `UPDATE inventory SET item_name = ?, type = ?, size = ?, quantity = ?, cost = ?, project = ?, is_dirty = 1 WHERE id = ?`,
        [itemName, itemType, size, quantity, cost, project, id],
      );
    } else {
      exec(
        `INSERT INTO inventory (item_name, type, size, quantity, cost, project, is_dirty) VALUES (?, ?, ?, ?, ?, ?, 1)`,
        [itemName, itemType, size, quantity, cost, project],
      );
      id = lastInsertId();
    }
  });
  const rows = all<SqlRow>(
    "SELECT id, item_name, type, size, quantity, cost, project FROM inventory WHERE id = ?",
    [id],
  );
  if (!rows[0]) throw new Error("That inventory row was not found on this PC.");
  return mapInventory(rows[0]);
}

function deleteInventoryLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM inventory WHERE id = ?", [id]);
  });
}

function moveInventoryLocal(id: number, project: string): InventoryRow {
  const rows = all<SqlRow>(
    "SELECT id, item_name, type, size, quantity, cost, project FROM inventory WHERE id = ?",
    [id],
  );
  if (!rows[0]) throw new Error("That inventory row was not found on this PC.");
  const row = mapInventory(rows[0]);
  row.project = trim(project);
  return saveInventoryLocal(row);
}

function mapLogistics(row: SqlRow): LogisticsRow {
  return {
    id: num(row, "id"),
    project: text(row, "project"),
    vehicleNumber: text(row, "vehicle_number"),
    invoiceNumber: text(row, "invoice_number"),
    startDate: text(row, "start_date"),
    reachDate: text(row, "reach_date"),
  };
}

function listLogisticsLocal(): LogisticsRow[] {
  return all<SqlRow>(
    `SELECT id, project, vehicle_number, invoice_number, start_date, reach_date
     FROM logistics ORDER BY start_date DESC, id DESC`,
  ).map(mapLogistics);
}

function saveLogisticsLocal(payload: LogisticsRow): LogisticsRow {
  const vehicleNumber = requireText(payload.vehicleNumber, "Vehicle number");
  const project = trim(payload.project);
  const invoiceNumber = trim(payload.invoiceNumber);
  const startDate = trim(payload.startDate);
  const reachDate = trim(payload.reachDate);
  let id = payload.id ?? 0;
  withTransaction(() => {
    touchProject(project);
    if (id) {
      exec(
        `UPDATE logistics SET project = ?, vehicle_number = ?, invoice_number = ?, start_date = ?, reach_date = ?, is_dirty = 1 WHERE id = ?`,
        [project, vehicleNumber, invoiceNumber, startDate, reachDate, id],
      );
    } else {
      exec(
        `INSERT INTO logistics (project, vehicle_number, invoice_number, start_date, reach_date, is_dirty)
         VALUES (?, ?, ?, ?, ?, 1)`,
        [project, vehicleNumber, invoiceNumber, startDate, reachDate],
      );
      id = lastInsertId();
    }
  });
  const rows = all<SqlRow>(
    `SELECT id, project, vehicle_number, invoice_number, start_date, reach_date FROM logistics WHERE id = ?`,
    [id],
  );
  if (!rows[0]) throw new Error("That logistics row was not found on this PC.");
  return mapLogistics(rows[0]);
}

function deleteLogisticsLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM logistics WHERE id = ?", [id]);
  });
}

function moveLogisticsLocal(id: number, project: string): LogisticsRow {
  const rows = all<SqlRow>(
    `SELECT id, project, vehicle_number, invoice_number, start_date, reach_date FROM logistics WHERE id = ?`,
    [id],
  );
  if (!rows[0]) throw new Error("That logistics row was not found on this PC.");
  const row = mapLogistics(rows[0]);
  row.project = trim(project);
  return saveLogisticsLocal(row);
}

function mapDocument(row: SqlRow): DocumentRow {
  return {
    id: num(row, "id"),
    name: text(row, "name"),
    path: text(row, "path"),
    linkedType: text(row, "linked_type"),
    linkedId: text(row, "linked_id"),
    createdAt: text(row, "created_at"),
  };
}

function listDocumentsLocal(): DocumentRow[] {
  return all<SqlRow>(
    `SELECT id, name, path, linked_type, linked_id, created_at FROM documents ORDER BY created_at DESC, id DESC`,
  ).map(mapDocument);
}

function saveDocumentLocal(payload: DocumentRow): DocumentRow {
  const name = trim(payload.name);
  const path = trim(payload.path);
  if (!name && !path) throw new Error("Name or file path is required.");
  const rawType = trim(payload.linkedType).toLowerCase();
  let linkedType = "";
  if (rawType === "voucher" || rawType === "project" || rawType === "po") linkedType = rawType;
  else if (rawType) throw new Error("Link type must be voucher, project, or PO.");
  const linkedId = trim(payload.linkedId);
  const storedName = name || path;
  let id = payload.id ?? 0;
  withTransaction(() => {
    if (id) {
      exec(
        `UPDATE documents SET name = ?, path = ?, linked_type = ?, linked_id = ?, is_dirty = 1 WHERE id = ?`,
        [storedName, path, linkedType, linkedId, id],
      );
    } else {
      exec(
        `INSERT INTO documents (name, path, linked_type, linked_id, created_at, is_dirty)
         VALUES (?, ?, ?, ?, datetime('now'), 1)`,
        [storedName, path, linkedType, linkedId],
      );
      id = lastInsertId();
    }
  });
  const rows = all<SqlRow>(
    `SELECT id, name, path, linked_type, linked_id, created_at FROM documents WHERE id = ?`,
    [id],
  );
  if (!rows[0]) throw new Error("That document was not found on this PC.");
  return mapDocument(rows[0]);
}

function deleteDocumentLocal(id: number): void {
  withTransaction(() => {
    exec("DELETE FROM documents WHERE id = ?", [id]);
  });
}

function openDocumentLocal(id: number): string {
  const rows = all<SqlRow>("SELECT path FROM documents WHERE id = ?", [id]);
  const path = rows[0] ? text(rows[0], "path").trim() : "";
  if (!path) throw new Error("That document has no file path on this PC.");
  throw new Error(
    `This file stays on this PC at ${path}. T Books does not upload it. Open it from File Explorer.`,
  );
}

function listProjectsLocal(): string[] {
  return all<SqlRow>("SELECT project FROM projects ORDER BY project COLLATE NOCASE")
    .map((row) => text(row, "project"))
    .filter(Boolean);
}

function listVendorsLocal(): VendorRef[] {
  return all<SqlRow>("SELECT vendor, gst FROM vendors ORDER BY vendor COLLATE NOCASE")
    .map((row) => ({ vendor: text(row, "vendor"), gst: text(row, "gst") }))
    .filter((row) => row.vendor);
}

function searchOfficeLocal(query: string): SearchHit[] {
  const q = trim(query);
  if (!q) return [];
  const like = likePattern(q);
  const hits: SearchHit[] = [];
  for (const row of all<SqlRow>(
    `SELECT vendor, gst FROM vendors WHERE vendor LIKE ? ESCAPE '\\' OR gst LIKE ? ESCAPE '\\'
     ORDER BY vendor COLLATE NOCASE LIMIT 20`,
    [like, like],
  )) {
    hits.push({
      kind: "vendor",
      id: text(row, "vendor"),
      title: text(row, "vendor"),
      subtitle: text(row, "gst"),
    });
  }
  for (const row of all<SqlRow>(
    `SELECT project FROM projects WHERE project LIKE ? ESCAPE '\\' ORDER BY project COLLATE NOCASE LIMIT 20`,
    [like],
  )) {
    const project = text(row, "project");
    hits.push({ kind: "project", id: project, title: project, subtitle: "" });
  }
  for (const row of all<SqlRow>(
    `SELECT id, item_name, type, size FROM inventory
     WHERE item_name LIKE ? ESCAPE '\\' OR type LIKE ? ESCAPE '\\' OR project LIKE ? ESCAPE '\\'
     ORDER BY item_name COLLATE NOCASE LIMIT 20`,
    [like, like, like],
  )) {
    hits.push({
      kind: "inventory",
      id: String(num(row, "id")),
      title: text(row, "item_name"),
      subtitle: [text(row, "type"), text(row, "size")].filter(Boolean).join(" · "),
    });
  }
  for (const row of all<SqlRow>(
    `SELECT id, vehicle_number, project, invoice_number FROM logistics
     WHERE vehicle_number LIKE ? ESCAPE '\\' OR invoice_number LIKE ? ESCAPE '\\' OR project LIKE ? ESCAPE '\\'
     ORDER BY vehicle_number COLLATE NOCASE LIMIT 20`,
    [like, like, like],
  )) {
    const id = String(num(row, "id"));
    const vehicle = text(row, "vehicle_number");
    hits.push({
      kind: "logistics",
      id,
      title: vehicle || `Trip ${id}`,
      subtitle: [text(row, "project"), text(row, "invoice_number")].filter(Boolean).join(" · "),
    });
  }
  return hits;
}

async function call<T>(command: string, local: () => T, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) return invokeCommand<T>(command, args);
  return local();
}

export async function listSalesPo(): Promise<SalesPo[]> {
  return call("list_sales_po", listSalesPoLocal);
}
export async function getSalesPo(id: number): Promise<SalesPo> {
  return call("get_sales_po", () => getSalesPoLocal(id), { id });
}
export async function saveSalesPo(payload: SalesPoSave): Promise<SalesPo> {
  return call("save_sales_po", () => saveSalesPoLocal(payload), { payload });
}
export async function deleteSalesPo(id: number): Promise<void> {
  return call("delete_sales_po", () => deleteSalesPoLocal(id), { id });
}
export async function listPurchasePo(): Promise<PurchasePo[]> {
  return call("list_purchase_po", listPurchasePoLocal);
}
export async function getPurchasePo(id: number): Promise<PurchasePo> {
  return call("get_purchase_po", () => getPurchasePoLocal(id), { id });
}
export async function savePurchasePo(payload: PurchasePoSave): Promise<PurchasePo> {
  return call("save_purchase_po", () => savePurchasePoLocal(payload), { payload });
}
export async function deletePurchasePo(id: number): Promise<void> {
  return call("delete_purchase_po", () => deletePurchasePoLocal(id), { id });
}
export async function listPurchasePayments(poNumber?: string | null): Promise<PurchasePayment[]> {
  return call("list_purchase_payments", () => listPurchasePaymentsLocal(poNumber), {
    poNumber: poNumber ?? null,
  });
}
export async function savePurchasePayment(payload: PurchasePaymentSave): Promise<PurchasePayment> {
  return call("save_purchase_payment", () => savePurchasePaymentLocal(payload), { payload });
}
export async function deletePurchasePayment(id: number): Promise<void> {
  return call("delete_purchase_payment", () => deletePurchasePaymentLocal(id), { id });
}
export async function submitOffice(kind: string, key: string): Promise<CasOutcome> {
  return call(
    "submit_office",
    () => {
      throw new Error("Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.");
    },
    { kind, key },
  );
}
export async function getTrial(fy?: string | null, asOf?: string | null): Promise<TrialBalance> {
  return call("get_trial", () => getTrialLocal(fy, asOf), { fy: fy ?? null, asOf: asOf ?? null });
}
export async function listFy(): Promise<string[]> {
  return call("list_fy", listFyLocal);
}
export async function listHrPeople(): Promise<HrPerson[]> {
  return call("list_hr_people", listHrPeopleLocal);
}
export async function saveHrPerson(payload: HrPerson): Promise<HrPerson> {
  return call("save_hr_person", () => saveHrPersonLocal(payload), { payload });
}
export async function deleteHrPerson(id: number): Promise<void> {
  return call("delete_hr_person", () => deleteHrPersonLocal(id), { id });
}
export async function listHrPayroll(): Promise<PayrollRow[]> {
  return call("list_hr_payroll", listHrPayrollLocal);
}
export async function saveHrPayroll(payload: PayrollRow): Promise<PayrollRow> {
  return call("save_hr_payroll", () => saveHrPayrollLocal(payload), { payload });
}
export async function deleteHrPayroll(id: number): Promise<void> {
  return call("delete_hr_payroll", () => deleteHrPayrollLocal(id), { id });
}
export async function listInventory(query?: string): Promise<InventoryRow[]> {
  return call("list_inventory", () => listInventoryLocal(query), { query: query ?? null });
}
export async function saveInventory(payload: InventoryRow): Promise<InventoryRow> {
  return call("save_inventory", () => saveInventoryLocal(payload), { payload });
}
export async function deleteInventory(id: number): Promise<void> {
  return call("delete_inventory", () => deleteInventoryLocal(id), { id });
}
export async function moveInventory(id: number, project: string): Promise<InventoryRow> {
  return call("move_inventory", () => moveInventoryLocal(id, project), { id, project });
}
export async function listLogistics(): Promise<LogisticsRow[]> {
  return call("list_logistics", listLogisticsLocal);
}
export async function saveLogistics(payload: LogisticsRow): Promise<LogisticsRow> {
  return call("save_logistics", () => saveLogisticsLocal(payload), { payload });
}
export async function deleteLogistics(id: number): Promise<void> {
  return call("delete_logistics", () => deleteLogisticsLocal(id), { id });
}
export async function moveLogistics(id: number, project: string): Promise<LogisticsRow> {
  return call("move_logistics", () => moveLogisticsLocal(id, project), { id, project });
}
export async function listDocuments(): Promise<DocumentRow[]> {
  return call("list_documents", listDocumentsLocal);
}
export async function saveDocument(payload: DocumentRow): Promise<DocumentRow> {
  return call("save_document", () => saveDocumentLocal(payload), { payload });
}
export async function deleteDocument(id: number): Promise<void> {
  return call("delete_document", () => deleteDocumentLocal(id), { id });
}
export async function openDocument(id: number): Promise<string> {
  return call("open_document", () => openDocumentLocal(id), { id });
}
export async function searchOffice(query: string): Promise<SearchHit[]> {
  return call("search_office", () => searchOfficeLocal(query), { query });
}
export async function listProjects(): Promise<string[]> {
  return call("list_projects", listProjectsLocal);
}
export async function listVendors(): Promise<VendorRef[]> {
  return call("list_vendors", listVendorsLocal);
}

export function parseFinite(raw: string, field: string): number {
  const t = raw.trim();
  if (!t) return 0;
  const n = Number(t);
  if (!Number.isFinite(n)) throw new Error(`${field} must be a number.`);
  return n;
}
