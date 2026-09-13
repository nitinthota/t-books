import assert from "node:assert/strict";
import { test } from "node:test";
import { LOOPBOOK_NAV, WINDOWS_HIVE_KINDS, SECRET_COLUMN_RE } from "../../src/lib/t-books/modules.ts";
import { HIVE_KINDS, liveTabName, salesPoHiveKey, tabName } from "../../src/lib/t-books/hive.ts";

test("T Books nav is 1:1 with Loopbook modes", () => {
  assert.deepEqual(
    LOOPBOOK_NAV.map((m) => m.label),
    [
      "Board",
      "Vouchers",
      "Finance",
      "Purchase",
      "Sales",
      "Vendors",
      "Projects",
      "Inventory",
      "Logistics",
      "HR",
      "Documents",
      "Duplicates",
      "Explorer",
      "Rules",
      "Access",
      "System",
    ],
  );
  assert.equal(LOOPBOOK_NAV.length, 16);
});

test("Windows hive kinds cover every office module plus Access and vouchers", () => {
  assert.deepEqual([...WINDOWS_HIVE_KINDS], [...HIVE_KINDS]);
  assert.equal(tabName("sales_po"), "Sales_PO");
  assert.equal(liveTabName("voucher"), "Voucher register");
  assert.equal(salesPoHiveKey("PO-1", "CUST_01"), "PO-1@CUST_01");
});

test("explorer secret column matcher hides passwords", () => {
  assert.equal(SECRET_COLUMN_RE.test("password_hash"), true);
  assert.equal(SECRET_COLUMN_RE.test("vendor"), false);
});
