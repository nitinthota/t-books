/** 1:1 Loopbook chrome. Labels and order match loopbook `app-shell` NAV. */

export const LOOPBOOK_NAV = [
  { id: "board", label: "Board", section: "Books" },
  { id: "vouchers", label: "Vouchers" },
  { id: "finance", label: "Finance" },
  { id: "purchase", label: "Purchase", section: "Work" },
  { id: "sales", label: "Sales" },
  { id: "vendors", label: "Vendors" },
  { id: "projects", label: "Projects" },
  { id: "inventory", label: "Inventory" },
  { id: "logistics", label: "Logistics" },
  { id: "hr", label: "HR" },
  { id: "documents", label: "Documents", section: "Control" },
  { id: "duplicates", label: "Duplicates" },
  { id: "explorer", label: "Explorer" },
  { id: "rules", label: "Rules" },
  { id: "access", label: "Access" },
  { id: "system", label: "System" },
] as const;

export type LoopbookNavId = (typeof LOOPBOOK_NAV)[number]["id"];

/** Hive kinds Windows T Books can Submit. Views never call these. */
export const WINDOWS_HIVE_KINDS = [
  "access",
  "voucher",
  "purchase",
  "payment",
  "salary",
  "sales_po",
  "inventory",
  "logistics",
  "document",
] as const;

export const EXPLORER_TABLES = [
  "vouchers",
  "voucher_payments",
  "vendors",
  "projects",
  "inventory",
  "logistics",
  "sales_po",
  "sales_po_items",
  "purchase_po",
  "purchase_po_items",
  "purchase_payments",
  "hr_people",
  "hr_payroll",
  "documents",
  "pending_submit",
  "access_cache",
  "rules",
] as const;

export type ExplorerTable = (typeof EXPLORER_TABLES)[number];

export const SECRET_COLUMN_RE = /password|token|secret|private_key|bearer|credential/i;
