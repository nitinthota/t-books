# AGENT_LOGIC_LOG — T Books 10/10 pass (Auth excluded)

Logic-check agent reads this file. A module is GREEN only if core structure below matches the running code and `cargo test --lib` + `tsc` have no errors.

## Agent P — Purchase

Core:
- Bill key = PUR-n. Pay key = PAY-n. Posted PUR-n / PAY-n / PUR-n-01 never rewritten.
- Type labels on screen: Techsol Purchase (`po` / contract) and Project Expenses (`non_po` / simple).
- Submit bill only when `shouldPostPurchaseBill({ goodsReceived, grandPaise, converted:false })`.
- Merge persist: `merge_onto_purchase(po, voucherNos)` writes `vouchers.linked_po` + `dirty=1` after `mergeBlockedReason` is null.
- Unmerge only `canUnmergeVoucher(source)` (import payments).
- Views: SQLite only.

Status: IMPLEMENTED — awaiting compile green.

## Agent S — Sales

Core:
- One calcPo for preview and persist.
- No duplicate PO on same project. Same number, different projects allowed.
- Submit key = `PO@project`.
- Project rollup customer PO = sum of sales totals on that project.

Status: UNCHANGED this pass (already ported). GREEN if tests still pass.

## Agent V — Vouchers

Core:
- Open uses `get_voucher_full` → `get_voucher`.
- Integer column A. No 20.1.
- Merge link is local dirty; Submit still one voucher row.

Status: IMPLEMENTED helper wire. GREEN if tests pass.

## Agent M — Vendors / Projects / Masters

Core:
- `list_vendors` returns vendor, gst, bank, accountNumber, ifsc from SQLite in one query.
- `formatBankLabel` on Vendors screen.
- Projects rollup: Vouchers, Billed, Paid, Due, Customer PO.

Status: IMPLEMENTED.

## Agent H — Hive / sheets hygiene

Core:
- Views never call Google.
- `fetch_sheet_values` is a default-book wrapper; live path is `fetch_values_on`.
- Raw archive never written.

Status: NO behavior change. Dead wrapper may still warn on desktop feature; not a hive-law break.

## Logic-check agent

Flag GREEN only when:
1. `npm run typecheck` clean
2. `cargo test --lib` 126+ passed (masters tests extra)
3. No view command calls Google
4. Posted numbers not rewritten

## Logic-check agent (local, 2026-09-17)

Gate:
- `cargo test --lib` → 130 passed, 0 failed, **no warning: lines**
- `npm run typecheck` → clean after `npm ci`
- Views never call Google
- Posted PUR-n / PAY-n / PUR-n-01 refused as merge targets (`is_child_pay_number` + dotted `20.1`)

**Flag: GREEN — allowed to push and then build Setup.exe**
Auth excluded. Do not dispatch installer until GitHub `check` job on this commit is green.

## Agent V/H — Voucher open + hive hygiene
Core:
- get_voucher_full used by save_voucher (lib path)
- fetch_sheet_values allowed dead; views never call Google
- merge refuses PUR-n-01
- Status: GREEN

## Agent P — Purchase (local)
Core structure:
- Type labels: Techsol Purchase | Project Expenses (storage contract/simple)
- Submit bill only if shouldPostPurchaseBill
- PAY Submit separate key
- Status: GREEN — files touched: src/components/t-books/purchase-screen.tsx

## Agent M — Merge persist + Vendors SQLite
Core structure:
- list_vendors one query: vendor, gst, bank, accountNumber, ifsc
- merge_onto_purchase writes vouchers.linked_po + dirty=1
- unmerge only import
- Status: GREEN
