# PARITY_MATRIX — Loopbook rules vs T Books

**AI:** do not merge the Loopbook repo into T Books. Read `docs/AI_DO_NOT_MERGE.md` first.

Source: `nitinthota/loopbook` main `a7639017` (`src/lib/erp/*` + route strings).
Target: `nitinthota/t-books`.
Dummy ids only: CUST_01, VEND_02, VOUCHER_1001, PUR-0001, PAY-0001, SAL-0001.

Columns: name | source file | t-books file | ported | missing | drifted

## C — shared contracts

| name | source file | t-books file | ported | missing | drifted |
| --- | --- | --- | --- | --- | --- |
| rupeesToPaise | loopbook/src/lib/erp/money.ts | src/lib/t-books/business_rules/money.ts + src-tauri/.../money.rs | yes | | |
| formatInr / formatRupees / formatQty / paiseToRupees | money.ts | money.ts + money.rs | yes | | |
| calcPo | po-calc.ts | business_rules/po-calc.ts + calc_po.rs | yes | | |
| calcPoItem | po-calc.ts | po-calc.ts + calc_po.rs | yes | | |
| clampPct | po-calc.ts | po-calc.ts + calc_po.rs | yes | | |
| paymentTermDays | po-calc.ts | po-calc.ts + calc_po.rs | yes | | |
| calcPayroll | po-calc.ts (Loopbook lives here) | po-calc.ts + hr-payroll twin | yes | | |
| isBlankPoItem | purchase-status.ts | purchase-status.ts | yes | | |
| poListMoney | purchase-status.ts | purchase-status.ts | yes | | |
| nextPurchaseNumber (counts PUR- and PURC:) | purchase-status.ts | purchase-status.ts + alloc.rs | yes | | |
| nextPaymentNumber workspace-wide | purchase-status.ts | purchase-status.ts + alloc.rs | yes | | |
| allocateVoucherSerial | voucher-data.ts | voucher-data.ts + alloc.ts | yes | | |
| paymentStatus | voucher-data.ts | voucher-data.ts (status.ts re-exports) | yes | | |
| taxFlagFor / isNaTaxInv / isDottedChildSerial / normalizeVoucherNo | voucher-data.ts | voucher-data.ts | yes | | |
| classifyPurchasePayment | purchase-status.ts | purchase-status.ts | yes | | |
| purchasePayStatus | purchase-status.ts | purchase-status.ts | yes | | |
| allocMethodForPurchase | purchase-status.ts | purchase-status.ts | yes | | |
| paymentMatchesFilters | purchase-status.ts | purchase-status.ts | yes | | |
| taxInvoiceMissing | purchase-status.ts | purchase-status.ts | yes | | |
| shouldPostPurchaseBill | purchase-status.ts | purchase-status.ts | yes | | |
| paymentOverTotal | purchase-status.ts | purchase-status.ts | yes | | |
| mergeBlockedReason | purchase-status.ts | purchase-status.ts | yes | | |
| pickDefaultMergeTarget | purchase-status.ts | purchase-status.ts | yes | | |
| canUnmergeVoucher / lastVoucherUnmerge | purchase-status.ts | purchase-status.ts | yes | | |
| deleteReasonOk | purchase-status.ts | purchase-status.ts | yes | | |
| isHttpUrl / documentLinkAudit | purchase-status.ts | purchase-status.ts | yes | | |
| formatBankLabel | purchase-status.ts | purchase-status.ts | yes | | |
| fyRange / fyBounds / currentIndianFy | fy.ts | fy.ts (fyRange aliases fyBounds) | yes | | |
| calcSalarySlip | hr-payroll.ts | hr-payroll.ts | yes | | |
| rbac gates | rbac.ts | rbac.ts (T Books roles owner/admin/operator) | yes | | Loopbook roles are owner/operator/viewer |

## Naming lock — chrome

| name | source file | t-books file | ported | missing | drifted |
| --- | --- | --- | --- | --- | --- |
| Product T Books | — | constants.ts | yes | | |
| Installer T-Books-Setup.exe | — | ops.rs / download_update | yes | | |
| Data %LOCALAPPDATA%\\T-Books | — | constants.ts DATA_FOLDER_NAME | yes | | |
| Offline banner exact | — | constants.ts OFFLINE_BANNER | yes | | |
| Unsaved triad Save & go back / Discard / Cancel | — | unsaved-guard.tsx | yes | | |
| Board button Refresh from Google | — | board-shell.tsx | yes | | |
| Editor buttons Save / Submit / Back | — | module screens | yes | | |
| Conflict Reload / stay dirty | — | submit-conflict.tsx | yes | | |
| Dirty refresh Keep local / Discard local | — | board-shell.tsx | yes | | |
| Access columns Name / Email / Role / Active | — | access-screen.tsx | yes | | |
| Roles owner / admin / operator | — | types.ts | yes | | |
| Active Yes / No | — | access-screen.tsx | yes | | |
| Nav Board Vouchers Finance Purchase Sales Vendors Projects Inventory Logistics HR Documents Duplicates Explorer Rules Access System | loopbook app-shell | desktop-layout.tsx LOOPBOOK_NAV | yes | | Windows hive on System |


## V — Vouchers

| name | source file | t-books file | ported | missing | drifted |
| --- | --- | --- | --- | --- | --- |
| Filter Missing Tax Invoice | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Filter NA needs comment | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Filter Pending | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Filter Partial | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Filter Advance | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Filter Full | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Multi-select filters | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Columns # vendor project value paid still to pay status dirty | vouchers.tsx | vouchers-screen.tsx | yes | | |
| Skip dummy VOUCHER_* SAMPLE% CUST_* VEND_* | voucher-data.ts | payment.ts shouldSkipSheetRow | yes | | |
| Skip 20.1 / coerce 20.0 → 20 | voucher-data.ts | normalizeVoucherNo / isDottedChildSerial | yes | | |
| Open header vendor project tax inv GST bank A/c IFSC date description remarks | voucher.$id.tsx | vouchers-screen.tsx + get_voucher | yes | | |
| MoneyHero value / paid / still to pay | voucher-form | money-hero.tsx | yes | | |
| Add payment on this voucher max 5 | voucher-form | voucher_edit.rs + vouchers-screen | yes | | |
| Reject 6th with real message | voucher-form | voucher_edit.rs | yes | | |
| Never create 20.1 | voucher-data.ts | voucher_edit.rs | yes | | |
| NA tax without comment → NA needs comment | voucher-data.ts taxFlagFor | summarize_voucher | yes | | |
| New voucher integer column A | voucher.new.tsx | next_voucher_number + save_voucher | yes | | |
| Save dirty / Submit CAS / Back top-left | hive law | save_voucher + submit_voucher | yes | | |
| View command list_vouchers / get_voucher | — | lib.rs | yes SQLite only | | |

## P — Purchase

| name | source file | t-books file | ported | missing | drifted |
| --- | --- | --- | --- | --- | --- |
| Filters Advance / Partial / Fully Paid / Missing Tax Invoice | purchase.tsx | purchase-screen.tsx | yes | | |
| Columns PUR vendor project type tax inv goods received grand paid balance status dirty | purchase.tsx | purchase-screen.tsx | yes | | |
| Sections Basic / Contract / Items / Summary | purchase-form.tsx | purchase-screen.tsx | yes | | |
| Type po = Techsol Purchase | purchase-status.ts | purchase-screen.tsx | yes | | storage still accepts contract/simple aliases |
| Type non_po = Project Expenses | purchase-status.ts | purchase-screen.tsx | yes | | |
| Shared PUR- sequence | purchase-status.ts | nextPurchaseNumber | yes | | |
| Items + live calcPo + negatives | po-calc.ts | po-items-editor + calcPo | yes | | |
| Blank unnamed zero-rate ignored | isBlankPoItem | po-items-editor | yes | | |
| Over-pay blocked paymentOverTotal | purchase-status.ts | office save payment | yes | | |
| Delete reason >= 3 chars | deleteReasonOk | purchase-screen.tsx | yes | | |
| PAY alloc Advance / Against Ref / On Account | purchase-status.ts | purchase-screen.tsx | yes | | |
| Default alloc from goodsReceived | allocMethodForPurchase | office.ts | yes | | |
| shouldPostPurchaseBill | purchase-status.ts | purchase-status.ts | yes | UI does not hide Submit on unreceived | |
| Merge same vendor only | mergeBlockedReason | purchase-status.ts | rule yes | merge pane not on desktop | drifted UI |
| Unmerge only linked import payments | canUnmergeVoucher | purchase-status.ts | rule yes | unmerge pane | drifted UI |
| Keep posted PUR-n-01 | keepPostedNumber | alloc.ts | yes | | |
| Document link http/https | isHttpUrl | documents-screen.tsx | yes | | |
| formatBankLabel | purchase-status.ts | purchase-status.ts | yes | vendor detail bank line thin | |
| Save bill and Submit PAY different keys | hive | submit_office purchase vs payment | yes | | |

## S — Sales

| name | source file | t-books file | ported | missing | drifted |
| --- | --- | --- | --- | --- | --- |
| List PO project client GST subtotal GST amt grand received balance dirty | sales.tsx | sales-screen.tsx | yes | | |
| poListMoney received vs grand | purchase-status.ts | sales-screen.tsx | yes | | |
| One PoEditor Basic / Contract / Items / Summary | po-editor.tsx | sales-screen.tsx | yes | | |
| No duplicate PO on same project | sales-po-form | office save_sales_po | yes | | |
| Same number different projects allowed | sales-po-form | office | yes | | |
| Contract vs project is a field | po-editor.tsx | sales-screen kind toggle | yes | | |
| Received may be negative | po-calc.ts | sales-screen | yes | | |
| Save dirty / Submit PO@project | hive | submit_office sales_po | yes | | Sales_PO tab |
| calcPo preview equals persist | po-calc.ts | previewPo + saveSalesPo | yes | | |

## Hive commands that touch Sheets (must stay off views)

| command | touches Sheets | view? |
| --- | --- | --- |
| refresh_access | yes Access tab | no — explicit Refresh |
| write_access_row | yes Access tab | no — owner write |
| refresh_vouchers / force_refresh_vouchers | yes Voucher_Raw_Data | no — Board Refresh |
| submit_voucher / reload_voucher | yes one voucher row | no |
| submit_office | yes one hive row by kind | no |
| hive_status | yes read tab presence | no — System |
| bootstrap_hive_tab | yes headers only | no — owner |
| retry_pending_submit | yes one outbox row | no |
| login / inspect / set_password | Access read only when online | login, not a books view |
| list_vouchers get_voucher save_voucher list_* get_* save_* search_office get_trial list_duplicates explore_table list_rules | SQLite only | yes |

Views calling Google: none after this pass.

## Still later (not V/P/S stop condition)

| name | note |
| --- | --- |
| Merge / unmerge desktop pane | rule ported, pane missing |
| Project-screen money rollup | VP thin |
| Hive tabs Purchase Payments Payroll Sales_PO Inventory Logistics Documents | Windows System hive_status + owner bootstrap headers |
| HR SAL-n Refresh isolation | implemented locally; Payroll hive tab on Submit |
