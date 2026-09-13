# Logic gap — T Books vs Loopbook `@6e4ac1eb`

Cartographer (Agent 0). **No behavior changed.**

| Repo | Ref | Role |
|---|---|---|
| `nitinthota/t-books` | current `main` (`d7becdd`) | Desktop app |
| `nitinthota/loopbook` | `6e4ac1eb` (12 Sep 2026) | Business-rules reference only |

Loopbook screens, Neon, Vercel, Better Auth, and Google Sign-In as app login are **out of scope**. T Books roles stay `owner` / `admin` / `operator`. Map Loopbook `viewer` → fail-closed mutate (Agent 1).

Dummy tokens only in tests: `CUST_01`, `VEND_02`, `VOUCHER_1001`, `PUR-0001`, `PAY-0001`, `SAL-0001`.

---

## Status legend

| Status | Meaning |
|---|---|
| **ported** | Same named rule exists in Rust (and TS twin where required). Used by persist/preview. |
| **drifted** | Exists but differs in signature, copy, or edge cases. |
| **missing** | Loopbook function / hive contract not in T Books. |
| **n/a** | Loopbook web/auth/infra. Do not port. |

---

## A. Business rules

| Loopbook file | Function | T Books file | Status | Risk |
|---|---|---|---|---|
| `voucher-data.ts` | `parseAmount` | `payment.rs` `parse_amount` | ported | — |
| `voucher-data.ts` | `emptyPayment` | `payment.rs` `empty_payment` | ported | — |
| `voucher-data.ts` | `normalizeVoucherNo` (`20.0` → `20`) | `payment.rs` `normalize_voucher_no` | ported | — |
| `voucher-data.ts` | `isDottedChildSerial` (`20.1`) | no named fn; `parse_voucher_number` rejects non-integers | drifted | Medium — Agent 1 should port by name so 20.1 never becomes a row |
| `voucher-data.ts` | `paymentStatus` | `status.rs` `payment_status` | drifted | Medium — Loopbook rounds each side to paise then subtracts; T Books `js_round(invoice - payment)` |
| `voucher-data.ts` | `taxFlagFor` / `isNaTaxInv` / `statusTone` | `status.rs` | ported | — |
| `voucher-data.ts` | 5 PI/payment slots, TDS counts as paid | `payment.rs` `MAX_PAYMENT_BLOCKS` / `voucher_totals` | ported | — |
| `voucher-data.ts` | skip empty vendor / dotted child | `vouchers.rs` skip invalid column A | drifted | Medium — empty vendor with integer A still imports |
| `voucher-data.ts` | dummy serials (`VOUCHER_*`) | parse fails non-integer A | drifted | Low — Agent 8 wants explicit skip of `VOUCHER_*` / `SAMPLE%` |
| `voucher-data.ts` | `allocateVoucherSerial` | — | missing | High — hive-side next number |
| `voucher-data.ts` | `headerMoneyFromLines` / `countSheetPaymentLines` | — | missing | Low — import report polish |
| `voucher-data.ts` | `parseSheetDate` | not a named rule | missing | Low |
| `money.ts` | `rupeesToPaise` / `paiseToRupees` / `formatInr` / `formatRupees` | `money.rs` | ported | — |
| `money.ts` | `formatQty` | — | missing | Low |
| `po-calc.ts` | `clampPct` / `paymentTermDays` / `calcPoItem` / `calcPo` | `calc_po.rs` | drifted | Medium — Loopbook `item_name` on lines; T Books description-only. Negatives allowed in both |
| `po-calc.ts` | `calcPayroll` (salary, net, CTC, PF total) | `calc_po.rs` `calc_payroll` (PF+TDS only) | drifted | High — salary/net/CTC not in the one function |
| `purchase-status.ts` | whole module (`PUR-n`, `PAY-n`, Advance / Against Ref / On Account, `classifyPurchasePayment`, merge/unmerge, `nextPurchaseNumber`, `nextPaymentNumber`, keep posted `PUR-n-01`) | — | **missing** | **Critical** |
| `fy.ts` | `currentIndianFy` / `fyBounds` / `fyOptions` / `clampAsOf` / `trialClosing` / `signedDrCr` / `trialBalanced` | — | **missing** | **High** |
| `hr-payroll.ts` | `nextSalaryNumber` / `calcSalarySlip` / `outstandingAdvancePaise` / `salaryVoucherLines` / `linesBalance` / `salaryPeriodTaken` / `keepPostedPayNumber` / `pickPayrollAsOf` | office payroll is a local month row (PF+TDS+total) | **missing** | **Critical** |
| `rbac.ts` | `canWrite` / `requireWrite` / `requireAdmin` / roles `owner\|operator\|viewer` | `rbac.ts` `canEditBooks` / `canRefresh` / `canOpenAccess`; roles `owner\|admin\|operator` | drifted | Medium — keep T Books roles; map Loopbook viewer → no mutate; write gates not fail-closed on every office command |
| `workspace-access.ts` | `workspaceAccessDecision` | `auth.rs` + hardcoded owner | n/a | Web invite model. Do not port |
| — | hive `fp` + integer `rev` on every row | `source_hash` / `fingerprint` only; **no `rev`** | missing | **Critical** — clocks must not order writes |
| — | `pending_submit` outbox | no table | missing | **Critical** — crash mid-Submit / retry |
| — | number alloc from hive list + local dirty | none | missing | **High** — two PCs can mint the same PAY/PUR |

---

## B. Hive / Google (commands)

Hive law: **views never call Google.** Save = SQLite + dirty. Submit = one hive row, CAS.

### Commands that touch Google Sheets

| Tauri command | Tab | Direction | Allowed? |
|---|---|---|---|
| `login` | Access | read (prefetch if online) | Yes — “Login online → read Access only” |
| `set_password` | Access | read (prefetch if online) | Yes — same |
| `refresh_access` | Access | read whole tab | Yes — explicit |
| `refresh_vouchers` | `Voucher_Raw_Data` | read whole tab | Yes — explicit; dirty guard is **vouchers only** |
| `force_refresh_vouchers` | `Voucher_Raw_Data` | read, discard dirty vouchers | Yes — explicit discard |
| `submit_voucher` | `Voucher_Raw_Data` | **one row** write (CAS on `source_hash`) | Yes — explicit; integer column-A only |
| `reload_voucher` | `Voucher_Raw_Data` | **one row** read, replace local | Yes — conflict recovery |

Access **write** (owner add/disable one row): **no Tauri command.** Gap.

### Commands that do **not** touch Sheets (views / local)

`inspect`, `current_session`, `logout`, `local_data_dir`, `get_ops_info`, backup/restore/logs/auto-backup, `check_for_updates` / `download_update` (GitHub, not Sheets), `get_access_list`, `get_app_status`, `get_dirty_vouchers`, `get_voucher_summary`, `list_vouchers`, `get_voucher`, all `list_*` / `get_*` / `save_*` / `delete_*` / `move_*` for sales, purchase, HR, inventory, logistics, documents, `preview_po`, `search_office`, `list_projects`, `list_vendors`, `open_document`.

**Confirmed:** no list/get/open/search command calls `sheets.rs`. Office CRUD is SQLite only.

### Hive contract vs current Submit

| Contract | T Books now |
|---|---|
| CAS: read row → match `fp`/`rev` → write | CAS on SHA `source_hash` only. **No `rev`.** |
| Success: clear dirty + outbox, store new fp/rev | Clears dirty; writes `source_hash`. No outbox, no rev |
| Different keys concurrent | Yes for two integer vouchers |
| Same key exclusive at Submit | Yes (fingerprint mismatch) |
| Offline Submit → real error; Save already local | Yes (`This PC is offline. Cannot submit to Google.`) |
| Conflict copy `{KEY} was just updated by another user.` | Drifted: `Voucher {n} was updated by another user.` (no “just”; voucher-only) |
| Refresh dirty guard lists **all dirty kinds** | Only integer vouchers |
| Missing hive tab → owner bootstrap **headers only** | Error string; no header bootstrap |
| Submit PUR / PAY / SAL / inventory / logistics / documents | **Missing** — those tables never leave this PC |
| Idempotent retry after crash | **Missing** — no outbox |

---

## C. Product surfaces (UI)

| Surface | Loopbook 12 Sep | T Books now | Status |
|---|---|---|---|
| Historical register | integer column A, 5 slots | Board + Vouchers | ported |
| New bills | `PUR-n` + goods received + tax inv + items + GST | Purchase = local PO (`simple` / `contract`) | missing |
| New bank payments | `PAY-n` linked Advance / Against Ref / On Account | payments sit on the integer voucher | missing |
| Purchase filters | Advance / Partial / Fully Paid / Missing Tax Inv | voucher filters only | missing |
| One PoEditor | sales + purchase + project meet by project | separate sales/purchase screens; frozen Simple PO kept | drifted |
| Indian FY trial | 1 Apr–31 Mar, must balance | none | missing |
| People + payroll | salary required if Active; `SAL-n`; one salary/person/month; advances recover; ledger lines balance | people + month PF/TDS row; no SAL-n; Active without salary allowed | missing |
| Inventory / logistics / documents | hive tabs later | local SQLite; Refresh does not delete them | ported (local); Submit-to-hive missing |
| Access | owner grant | owner can **read** Access; **cannot write** a sheet row from T Books | drifted |
| Offline banner | exact copy | exact copy | ported |
| Dirty / outbox on Board | count + last Refresh error | dirty voucher count; last error banner | drifted — no outbox; failed Refresh must never look like “0 vouchers” (already a warning path) |

---

## D. Closed / explicit defer

| Item | Decision |
|---|---|
| Loopbook screens, Neon, Vercel, Better Auth, Google Sign-In login | **Do not port** |
| OT / Yjs / Automerge / LWW on money | **Banned** |
| Splitting column A into `20.1` | **Banned** — historical row stays integer |
| Auto-refresh on page open | **Banned** |
| Views calling Sheets | **Banned** — currently held |
| Inventory/logistics hive Submit | Defer until those tabs exist; Refresh must not wipe local if tabs absent |
| Access write from T Books | Agent 6 — owner, online, **one row** |

---

## E. Agent handoff (do not start until this file is accepted)

1. **Agent 1** — add `purchase_status.rs`, `fy.rs`, `hr_payroll.rs`, `alloc.rs`; align `payment.rs` / `status.rs` / `calc_po.rs` / `money.rs`; dummy rows never allocate; posted numbers sacred.
2. **Agent 2** — `pending_submit` + `rev` + CAS; dirty guard all kinds; Hive trait for tests; Access/Voucher tabs bootstrap headers only.
3. **Agent 3** — PUR / PAY UI.
4. **Agent 4** — PoEditor, FY trial, SAL-n payroll.
5. **Agent 5** — Board dirty/outbox + failed Refresh banner.
6. **Agent 6** — Access write; write gates fail-closed.
7. **Agent 7** — inventory/logistics/documents Submit when tabs exist.
8. **Agents 8–9** — adversarial + 3-PC fake hive.
9. **Agent 10** — polish / 4 GB; no hive-test regressions.

**Definition of done for this file:** every **missing** / **drifted** row above is closed by a later agent or moved to § D with an owner.

---

## F. Landed this pass (Agents 1–10)

| Agent | Closed |
|---|---|
| **1** | `purchase_status`, `fy`, `hr_payroll`, `alloc`, `rbac` (viewer fail-closed). `is_dotted_child_serial`, `is_dummy_serial`, paise-each-side `payment_status`, `item_name` + salary/net/ctc `calcPayroll` (salary optional), `formatQty`. Dummy keys never allocate. Posted numbers sacred. Import skips empty vendor / dotted child / dummy serials. |
| **2** | `pending_submit` outbox; `hive_rev`; Hive trait (`get_row` / `cas_row` / `list_keys` / `ensure_tab`); CAS = fp+rev; conflict copy `{KEY} was just updated by another user. Reload and submit again.`; dirty guard lists all dirty kinds; missing tab → owner bootstrap headers only. Views still do not call Google. |
| **3** | PUR-n bills + PAY-n payments. Save = SQLite + dirty. Submit = one hive row. Goods received, tax invoice, Advance / Partial / Fully Paid / Missing Tax Inv filters. |
| **4** | PoEditor `item_name` column. Indian FY trial (1 Apr–31 Mar). SAL-n payroll; salary required if Active; one salary/person/month. |
| **5** | Board outbox of dirty keys. Failed Refresh keeps previous figures + “Refresh failed. Previous figures on this PC are kept.” |
| **6** | Access write: owner, online, one hive row. Viewer is not operator; login denied “Your role cannot open T Books.” Write gates fail-closed on save/delete/move. |
| **7** | Inventory / logistics / documents Submit when those tabs exist. Missing tab keeps local. Refresh does not wipe office rows. |
| **8–9** | 3-PC fake hive: same PUR conflicts, different keys concurrent, dummy skip, no LWW on money, Google fail keeps dirty. |
| **10** | Trial nav, mobile overflow-x nav, 150–220ms motion, dual-target preview. |

Dummy tokens only: `CUST_01`, `VEND_02`, `VOUCHER_1001`, `PUR-0001`, `PAY-0001`, `SAL-0001`.

Schema version **9**. `LOOPBOOK_LOGIC_VERSION` stays `"v1"`.

