export const ROLES = ["owner", "admin", "operator"] as const;
export type Role = (typeof ROLES)[number];

export type Session = {
  email: string;
  role: Role;
};

export type AuthInspect =
  | { kind: "set-password"; email: string }
  | { kind: "password"; email: string }
  | { kind: "denied"; message: string };

export type AuthOk = { ok: true; session: Session };
export type AuthErr = { ok: false; message: string };
export type AuthResult = AuthOk | AuthErr;

export type AccessRow = {
  name: string;
  email: string;
  role: string;
  active: string;
  accountType?: string;
  lastSynced: string;
};

export type AccessSnapshot = {
  rows: AccessRow[];
  lastSynced: string | null;
  credentialsFound: boolean;
};

export type AppStatus = {
  credentialsFound: boolean;
  lastSynced: string | null;
  voucherLastSynced?: string | null;
  lastError: string | null;
};

export type RowError = {
  voucherNumber: number | null;
  row: number;
  errorType: string;
};

export type RefreshOk = {
  kind: "ok";
  imported: number;
  skipped: number;
  errors: RowError[];
  warning: string | null;
};

export type DirtyKey = {
  kind: string;
  key: string;
};

export type RefreshDirty = {
  kind: "dirty";
  voucherNumbers: number[];
  keys?: DirtyKey[];
};

export type RefreshOutcome = RefreshOk | RefreshDirty;

export type SubmitOk = {
  kind: "ok";
  voucherNumber: number;
};

export type SubmitConflict = {
  kind: "conflict";
  voucherNumber: number;
  message: string;
};

export type SubmitOutcome = SubmitOk | SubmitConflict;

export type VoucherSummary = {
  count: number;
  lastSynced: string | null;
  pending: number;
  partial: number;
  full: number;
  advance: number;
  missingTax: number;
  dirty: number;
  remainingTotal: number;
};

export type VoucherListRow = {
  voucherNumber: number;
  vendor: string;
  project: string;
  taxInvoice: string;
  totalValue: number;
  totalPaid: number;
  remaining: number;
  status: string;
  isDirty: boolean;
  deskIntent?: string;
  deskDecision?: string;
  deskAt?: string;
};

export type PaymentView = {
  slot: number;
  piNo: string;
  piDate: string;
  piValue: number;
  paid: number;
  remaining: number;
  paymentDate: string;
  remarks: string;
  description?: string;
  tds?: number;
  paymentDetails?: string;
  paymentPercent?: number;
};

export type VoucherView = {
  voucherNumber: number;
  taxInvoice: string;
  vendor: string;
  project: string;
  gst: string;
  comments: string;
  totalValue: number;
  totalPaid: number;
  remaining: number;
  status: string;
  isDirty: boolean;
  payments: PaymentView[];
  voucherDate?: string;
  bank?: string;
  accountNumber?: string;
  ifsc?: string;
  description?: string;
  voucherType?: string;
  taxFlag?: string;
  deskIntent?: string;
  deskDecision?: string;
  deskAt?: string;
};

export type VoucherSave = {
  voucherNumber: number;
  voucherDate?: string;
  taxInvoice?: string;
  vendor: string;
  bank?: string;
  accountNumber?: string;
  ifsc?: string;
  gst?: string;
  project?: string;
  comments?: string;
  description?: string;
  voucherType?: string;
  payments: PaymentView[];
};

export type PoItemIn = {
  id?: number | null;
  itemName?: string;
  description: string;
  qty: number;
  rate: number;
  gstPct: number;
  amount: number;
};

export type PoPreview = {
  items: PoItemIn[];
  subtotal: number;
  gst: number;
  grandTotal: number;
};

export type SalesPoKind = "contract" | "project";

export type SalesPo = {
  id: number;
  project: string;
  poNumber: string;
  client: string;
  gst: string;
  totalValue: number;
  createdAt: string;
  updatedAt: string;
  kind?: SalesPoKind | string;
  receivedRupees?: number;
  paymentTermValue?: number;
  paymentTermUnit?: "days" | "months" | string;
  isDirty?: boolean;
  items: PoItemIn[];
};

export type SalesPoSave = {
  id?: number | null;
  project: string;
  poNumber: string;
  client: string;
  gst: string;
  items: PoItemIn[];
  kind?: SalesPoKind | string;
  receivedRupees?: number;
  paymentTermValue?: number;
  paymentTermUnit?: "days" | "months" | string;
};

export type PurchaseType = "contract" | "simple";

export type PurchasePo = {
  id: number;
  project: string;
  vendor: string;
  poNumber: string;
  type: PurchaseType;
  totalValue: number;
  createdAt: string;
  updatedAt: string;
  goodsReceived?: boolean;
  taxInvoiceNo?: string;
  taxInvoiceDate?: string;
  isDirty?: boolean;
  paidRupees?: number;
  payStatus?: string;
  items: PoItemIn[];
};

export type PurchasePoSave = {
  id?: number | null;
  project: string;
  vendor: string;
  poNumber: string;
  type: PurchaseType;
  totalValue: number;
  items: PoItemIn[];
  goodsReceived?: boolean;
  taxInvoiceNo?: string;
  taxInvoiceDate?: string;
};

export type PurchasePayment = {
  id?: number | null;
  payNumber: string;
  poNumber: string;
  vendor: string;
  project: string;
  amountRupees: number;
  allocMethod: string;
  payClass: string;
  missingTaxInvoice: boolean;
  payDate: string;
  remarks: string;
  isDirty?: boolean;
};

export type PurchasePaymentSave = {
  id?: number | null;
  payNumber: string;
  poNumber: string;
  vendor: string;
  amountRupees: number;
  allocMethod: string;
  payDate: string;
  remarks: string;
};

export type HrPerson = {
  id?: number | null;
  name: string;
  role: string;
  salary: number;
  active?: string;
};

export type PayrollRow = {
  id?: number | null;
  personId: number;
  personName: string;
  month: string;
  pfEmployee: number;
  pfCompany: number;
  pfTotal: number;
  tds: number;
  totalPaid: number;
  salaryNumber?: string;
  payKind?: string;
  salaryRupees?: number;
  recoveryRupees?: number;
  netRupees?: number;
  isDirty?: boolean;
};

export type InventoryRow = {
  id?: number | null;
  itemName: string;
  type: string;
  size: string;
  quantity: number;
  cost: number;
  project: string;
};

export type LogisticsRow = {
  id?: number | null;
  project: string;
  vehicleNumber: string;
  invoiceNumber: string;
  startDate: string;
  reachDate: string;
};

export type DocumentLinkType = "" | "voucher" | "project" | "po";

export type DocumentRow = {
  id?: number | null;
  name: string;
  path: string;
  linkedType: DocumentLinkType | string;
  linkedId: string;
  createdAt: string;
};

export type SearchHit = {
  kind: "vendor" | "project" | "inventory" | "logistics" | string;
  id: string;
  title: string;
  subtitle: string;
};

export type AccessWrite = {
  name: string;
  email: string;
  role: string;
  active: string;
};

export type PendingSubmit = {
  key: string;
  kind: string;
  payload: string;
  baseFp: string;
  baseRev: number;
  attempts: number;
  lastError: string;
};

export type TrialLine = {
  account: string;
  debitRupees: number;
  creditRupees: number;
};

export type TrialBalance = {
  fy: string;
  asOf: string;
  start: string;
  end: string;
  lines: TrialLine[];
  totalDebitRupees: number;
  totalCreditRupees: number;
  balanced: boolean;
};

export type CasOutcome =
  | { kind: "ok"; key: string; fp: string; rev: number }
  | { kind: "conflict"; key: string; message: string };

export type VendorRef = {
  vendor: string;
  gst: string;
  bank?: string;
  accountNumber?: string;
  ifsc?: string;
};

export type MergeReport = {
  poNumber: string;
  linked: number[];
  skipped: string[];
};

export type NavId =
  | "board"
  | "vouchers"
  | "finance"
  | "purchase"
  | "sales"
  | "vendors"
  | "projects"
  | "inventory"
  | "logistics"
  | "hr"
  | "documents"
  | "duplicates"
  | "merge"
  | "explorer"
  | "rules"
  | "access"
  | "system";
