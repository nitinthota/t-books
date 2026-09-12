# MODULE_GAP_VPS — Vouchers, Purchase, Sales

C0 cartographer. Compared **T Books `main` @ 15388d7** to **Loopbook `main` @ a7639017**.

Rules only from Loopbook. Screens rebuilt for desktop. Dummy ids only: CUST_01, VEND_02, VOUCHER_1001, PUR-0001, PAY-0001, SAL-0001.

Status: `ported` | `missing` | `drifted` | `deferred`

---

## Hive law check (do not change)

| Law | T Books now |
|---|---|
| Views read SQLite only | Holds. `list_vouchers`, `get_voucher`, office lists, sales lists never call Sheets. |
| Save = this PC + dirty | Holds for purchase/sales/office. Voucher header/payments **cannot be saved locally** from the UI (read-only open). |
| Submit = one row CAS fp+rev | Holds for voucher + office kinds (`submit_voucher`, `submit_office`). |
| Refresh stops on dirty | Holds for voucher dirty + `list_dirty_keys` (office kinds included in hive). UI conflict modal lists keys. |
| Posted PUR-n / PAY-n / PUR-n-01 sacred | Rules ported (`keepPostedPayNumber`, `isLegacyPayNumber`). UI does not yet keep PUR-n-01 child numbers visible. |
| calcPo one function | Ported in Rust + TS. Sales/purchase preview UI does not show live Summary (subtotal/GST/received/balance). |
| Offline banner exact text | Ported. |

---

## Commands that touch Google / Sheets

Only these desktop commands reach Sheets (or the hive wrapper that writes a tab). **No view command is in this list.**

| Command | When | Tab / kind |
|---|---|---|
| `login` / `set_password` | Online only, prefetch | Access (read) |
| `refresh_access` | Owner/admin trigger | Access (read; owner may bootstrap headers) |
| `write_access_row` | Owner online | Access (one row write) |
| `refresh_vouchers` | Manual Refresh | Voucher_Raw_Data (read) |
| `force_refresh_vouchers` | After Discard local | Voucher_Raw_Data (read) |
| `submit_voucher` | Explicit Submit | Voucher_Raw_Data (one row CAS) |
| `reload_voucher` | After conflict | Voucher_Raw_Data (one row read) |
| `submit_office` | Explicit Submit | Purchase / Payments / Payroll / Inventory / Logistics / Documents (one row CAS) |
| `check_for_updates` / `download_update` | Settings | GitHub releases, not the office sheet |

Views (`list_vouchers`, `get_voucher`, `list_sales_po`, `get_sales_po`, `list_purchase_po`, `get_purchase_po`, `list_purchase_payments`, Board summary, search, trial, vendors, projects) are SQLite only.

---

## A. Contract / rules (C1–C4)

| # | Loopbook behavior | File | T Books | Status |
|---|---|---|---|---|
| C1.1 | `calcPo` / `calcPoItem` / `clampPct` / `paymentTermDays` | `po-calc.ts` | `calc_po.rs` + `calc-po.ts` match | ported |
| C1.2 | `rupeesToPaise` / INR format | `money.ts` | `money.rs` + `money.ts` | ported |
| C1.3 | `poListMoney` (sales received vs purchase paid) | `purchase-status.ts` | TS+Rust present | ported |
| C1.4 | Negatives allowed on qty/rate | `po-calc.ts` | Same math | ported |
| C1.5 | Live preview uses same `calcPo` as persist | purchase-form / po-editor | Persist uses calc; **screens do not show live Summary** | drifted |
| C2.1 | `nextPurchaseNumber` PUR-NNNN + PURC: | `purchase-status.ts` | `alloc.rs` / `alloc.ts` | ported |
| C2.2 | `nextPaymentNumber` workspace PAY-NNNN | `purchase-status.ts` | same | ported |
| C2.3 | `allocateVoucherSerial` PREFIX-#### | `voucher-data.ts` | same | ported |
| C2.4 | Posted PAY-n / PUR-n-01 not rewritten | `keepPostedPayNumber`, `isLegacyPayNumber` | rules ported; UI allocation on Save is local-only | drifted |
| C2.5 | Submit allocate from hive list + local dirty; bump if taken | purchase-api | local next-* only; hive bump on Submit not complete | missing |
| C3.1 | `paymentStatus` paise-each-side | `voucher-data.ts` | `status.rs` / `status.ts` | ported |
| C3.2 | `taxFlagFor` / `isNaTaxInv` | `voucher-data.ts` | `payment.rs` | ported |
| C3.3 | `isDottedChildSerial` / `normalizeVoucherNo` (20.0→20, skip 20.1) | `voucher-data.ts` | parse path | ported |
| C3.4 | `classifyPurchasePayment` / `purchasePayStatus` / `allocMethodForPurchase` | `purchase-status.ts` | `purchase_status.rs` + TS | ported |
| C3.5 | `paymentMatchesFilters` / `taxInvoiceMissing` | `purchase-status.ts` | ported; list filters use them | ported |
| C3.6 | `shouldPostPurchaseBill` / `paymentOverTotal` | `purchase-status.ts` | rules exist; **UI never calls over-pay guard or post-bill gate** | missing |
| C3.7 | Header bill/paid if line reconstruction fails | `headerMoneyFromLines` | parse uses slots; header fallback not wired on open | missing |
| C4.1 | Submit CAS fp+rev | hive.rs / submit.rs | voucher + office | ported |
| C4.2 | Outbox `pending_submit` idempotent retry | hive.rs | present | ported |
| C4.3 | Refresh dirty guard all kinds | `list_dirty_keys` | present | ported |
| C4.4 | Sales PO Submit kind `sales_po` | — | office submit kinds: purchase/payment/salary/inventory/logistics/documents. **No sales_po hive tab** | missing |

---

## B. Module V — Vouchers

Loopbook: `vouchers.tsx`, `voucher.$id.tsx`, `voucher-form.tsx`, `voucher.new.tsx`, `voucher-data.ts`.

T Books: `vouchers-screen.tsx` (~8.6KB) list + read-only open + Submit.

| # | Loopbook behavior | Status | Notes |
|---|---|---|---|
| V1.1 | Filters: Missing Tax Invoice, NA needs comment, Pending, Partial, Advance, Full | missing | List has no filter chips / stat cards |
| V1.2 | Multi-select filters | missing | Loopbook is single-select stats; master prompt wants multi-select |
| V1.3 | Columns # vendor project value paid still-to-pay status dirty | ported | Dirty mark present |
| V1.4 | Virtualize | ported | `VirtualTable` |
| V1.5 | Dummy rows never listed | ported | parse skip |
| V1.6 | Status from amounts | ported | |
| V1.7 | Failed load = red banner not “0 vouchers” | ported | keeps previous rows |
| V1.8 | Search voucher/vendor/project/bank | missing | |
| V2.1 | Open: vendor project tax inv GST bank A/c IFSC date description remarks | drifted | Open shows vendor, project, tax inv, GST, comments. **No bank, A/c, IFSC, date, editable description/remarks** |
| V2.2 | MoneyHero value / paid / still to pay | ported | |
| V2.3 | Edit header fields + unsaved triad | missing | Open is read-only |
| V2.4 | Tax inv NA without comment → na_needs_comment | drifted | Flag computed on import; UI does not show or let you add the comment |
| V2.5 | Description visible | missing | Comments only |
| V3.1 | Up to 5 PI/payment blocks on THIS voucher | drifted | Blocks display; cannot add/edit |
| V3.2 | Add payment on same voucher, Save local + dirty | missing | No add-payment form |
| V3.3 | Slot fields: PI, value, description, TDS, paid, details, date, remaining, remarks | drifted | Table shows PI, value, paid, remaining, date only |
| V3.4 | Reject 6th block with real message | missing | No add path |
| V3.5 | Blank/odd money coerce to 0 | ported | parse |
| V4.1 | New voucher, integer column-A only | missing | No New voucher |
| V4.2 | Reject dotted child 20.1; coerce 20.0 → 20 | ported | on import only |
| V4.3 | Custom type as free-text label | missing | |
| V4.4 | Save local; Submit that sheet row | drifted | Submit exists; no local create/edit |
| V5.1 | Submit CAS voucher N + conflict modal | ported | |
| V5.2 | Refresh dirty lists voucher numbers | ported | |
| V5.3 | Tests: 20.1 skip, 20.0→20, NA flag, dummy skip, 6th pay, header totals, 2-PC CAS | drifted | parse + CAS tests exist; 6th-pay and new-voucher tests do not |

**Better-than-Loopbook already:** desktop Back, virtual list, dirty mark, Submit conflict modal. Missing: unsaved triad on voucher (nothing to save).

---

## C. Module P — Purchase

Loopbook: `purchase.tsx`, `purchase-form.tsx` (~59KB), `purchase-api.ts` (~75KB), `purchase-status.ts`.

T Books: `purchase-screen.tsx` (~16KB) short bill + amount-only PAY.

| # | Loopbook behavior | Status | Notes |
|---|---|---|---|
| P1.1 | Filters Advance / Partial / Fully Paid / Missing Tax Inv | ported | chips exist |
| P1.2 | Columns PUR vendor project type tax inv goods received grand paid balance status | drifted | List: PUR, vendor, status, total only |
| P1.3 | Virtualize + New / Open | ported | |
| P2.1 | Sections Basic / Contract / Items / Summary | missing | One flat “Bill” block |
| P2.2 | Type po = Techsol Purchase, non_po = Project Expenses | drifted | UI is contract/simple. Data field is `type`. Map required without losing rows |
| P2.3 | PUR blank allocates on save | ported | |
| P2.4 | Tax inv no+date, goods received, dates | drifted | tax + goods; no bill/start dates |
| P2.5 | Items name description qty rate GST%; live calcPo | drifted | items on contract; **no live Summary totals** |
| P2.6 | Blank unnamed zero-rate lines ignored (`isBlankPoItem`) | missing | not applied on save |
| P2.7 | Over-pay blocked (`paymentOverTotal`) | missing | amount-only PAY has no guard |
| P2.8 | Delete reason ≥ 3 chars | missing | Delete is one click |
| P2.9 | Save local + Submit PUR-n | ported | |
| P3.1 | PAY-n linked Advance / Against Ref / On Account | missing | allocMethod saved empty; no method UI |
| P3.2 | Default method from `allocMethodForPurchase` | missing | |
| P3.3 | Pay class computed not typed | drifted | backend classifies; UI does not show method |
| P3.4 | PAY editor fields date remarks | missing | amount only |
| P3.5 | Two PAY against same PUR both Submit | ported | separate keys |
| P3.6 | Keep posted PUR-n-01 | missing | no child-number display / keep path in UI |
| P4.1 | Merge same vendor; not on two purchases | missing | |
| P4.2 | Unmerge only linked import payments | missing | |
| P4.3 | `lastVoucherUnmerge` keep/delete/empty | missing | |
| P4.4 | Converted origin does not post second bill | missing | no converted origin field in UI |
| P5.1 | Document link http/https + audit line | missing | |
| P5.2 | Bank label bank/ac/ifsc/holder | missing | on bill |
| P6.1 | Submit PUR and PAY independently | ported | |
| P6.2 | CAS on same PUR | ported | tests |
| P6.3 | Simultaneous next-PAY bump | missing | local max only |
| P6.4 | Refresh must not delete purchase tables | ported | |

---

## D. Module S — Sales

Loopbook: `sales.tsx`, `po-editor.tsx`, `sales-po-form.tsx`, `po-calc.ts`. Contract + project POs, one PoEditor, received/balance, clone contract → project PO.

T Books: `sales-screen.tsx` (~8KB) one flat contract list. No Submit. No received.

| # | Loopbook behavior | Status | Notes |
|---|---|---|---|
| S1.1 | Columns PO project client GST subtotal GST amt grand received balance dirty | drifted | PO project client total only |
| S1.2 | `poListMoney` received vs grand | missing | no received field in sales UI |
| S1.3 | Virtualize New/Open | ported | |
| S2.1 | One PoEditor: Basic / Contract / Items / Summary | drifted | Basic + items; no Summary, no payment terms |
| S2.2 | Payment terms days or months → `paymentTermDays` | missing | |
| S2.3 | Live calcPo Summary | missing | |
| S2.4 | No duplicate PO on same project | missing | no check in save |
| S2.5 | Save local + dirty | ported | |
| S3.1 | Kind field contract vs project, same editor | missing | contract only |
| S3.2 | Clone contract → project PO on same project | missing | |
| S3.3 | Received can be negative | missing | no received |
| S4.1 | Project received = sum sales_pos on that project | missing | project screen is a thin list |
| S4.2 | Cost vs POs received on project from SQLite | missing | |
| S5.1 | Submit sales_po by PO+project | missing | no Submit button, no hive kind |
| S5.2 | CAS on same key | missing | |
| S5.3 | Refresh must not delete sales POs | ported | |
| S5.4 | Same PO number on different projects allowed | missing | no uniqueness rule yet |

---

## E. Cross-module

| # | Rule | Status |
|---|---|---|
| X.1 | Shared item grid + one calcPo | drifted | `po-items-editor` shared; Summary not shared |
| X.2 | Vendor/project suggest from SQLite | ported | purchase/sales |
| X.3 | Board outbox includes voucher + PUR + PAY + sales PO | drifted | dirty keys include office; sales_po not a hive kind |
| X.4 | Operator edit+Submit; no Access; owner/admin Refresh | drifted | Submit gated to owner/admin in `require_submit_role`. Master prompt: operator may Submit books. **Decide: current code is stricter.** |
| X.5 | No real vendor/GST/bank/amounts in code or logs | ported | |

---

## F. Test matrix vs now

| # | Case | Now |
|---|---|---|
| 1 | Offline Save works; Submit refused with real message | Purchase/sales Save yes. Voucher Save no. Submit offline errors. |
| 2 | Failed Refresh never paints empty | Holds |
| 3 | Dirty Refresh lists keys and waits | Holds |
| 4–5 | Keep local / Discard local | Holds for vouchers |
| 6 | A Submit PUR, B Refresh sees it | Not in v1 write-back for office tabs unless Submit ran; Refresh does not pull PUR |
| 7 | Both Save same PUR offline; A Submit; B Submit rejected | CAS test exists for MemoryHive |
| 8 | PAY-0001 and PAY-0002 same PUR both Submit | Supported if both saved |
| 9 | Simultaneous next PAY bump | missing |
| 10 | Voucher 20 dirty on A; B Refresh of clean PC | Holds |
| 11 | Hive hand-edit → fingerprint mismatch | voucher Submit |
| 12 | Crash mid-Submit outbox retry | hive tests |
| 13 | 20.1 never a voucher; dummy never books | parse tests |
| 14 | NA without comment flagged; junk qty → 0 | parse; UI does not surface NA flag |

---

## G. Agent order from here

1. **C1–C4 leftovers** — live Summary, hive+local allocate on Submit, over-pay + post-bill in UI, header fallback, `sales_po` hive kind.
2. **V** — filters, header edit, add payment (5 max), new integer voucher, NA comment.
3. **P** — sections, po/non_po labels, PAY method UI, over-pay, delete reason; merge after that.
4. **S** — kind field, received/balance, duplicate-PO-on-project, Submit `sales_po`, project money link.

Do not polish the current thin screens and call the module done.
