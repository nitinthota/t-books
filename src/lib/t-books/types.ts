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

export type RefreshDirty = {
  kind: "dirty";
  voucherNumbers: number[];
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
};

export type PoItemIn = {
  id?: number | null;
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

export type SalesPo = {
  id: number;
  project: string;
  poNumber: string;
  client: string;
  gst: string;
  totalValue: number;
  createdAt: string;
  updatedAt: string;
  items: PoItemIn[];
};

export type SalesPoSave = {
  id?: number | null;
  project: string;
  poNumber: string;
  client: string;
  gst: string;
  items: PoItemIn[];
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
};

export type HrPerson = {
  id?: number | null;
  name: string;
  role: string;
  salary: number;
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

export type VendorRef = {
  vendor: string;
  gst: string;
};

export type NavId =
  | "board"
  | "vouchers"
  | "vendors"
  | "projects"
  | "sales"
  | "purchase"
  | "hr"
  | "inventory"
  | "logistics"
  | "documents"
  | "settings"
  | "access";
