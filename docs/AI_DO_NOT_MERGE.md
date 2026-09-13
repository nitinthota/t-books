# For the next AI — what is missing, and why you must not merge Loopbook into T Books

Read this before copying code from `nitinthota/loopbook` into `nitinthota/t-books`.

T Books is a **Windows desktop app** (Tauri + SQLite on this PC). Google Sheet is a **hive** (clumsy central register). Loopbook is a **web ERP** (React + Postgres + Neon + Vercel + Better Auth).

Same **money rules**. Different **product**. You cannot merge the Loopbook repo into T Books. You may only **port named business rules**.

Dummy ids only: `CUST_01`, `VEND_02`, `VOUCHER_1001`, `PUR-0001`, `PAY-0001`, `SAL-0001`.

---

## 1. Never copy these. They will break T Books.

| Loopbook thing | Why it cannot go into T Books |
| --- | --- |
| Neon / Postgres / `@/lib/db` / migrations as login | T Books login is Access tab + local hash on this PC. Neon would reject every office PC. |
| Vercel / Better Auth / `authMiddleware` / `requireUserId` | Not used. Preview “dev user” is not an office user. |
| Google Sign-In as app login | Google is hive only (Access + sheet rows). App login is email + password on this PC. |
| Loopbook screens / CSS / routes as the UI | Spec: do not freeze Loopbook screens. Rebuild desktop chrome (left nav, Back, Save / Submit). |
| `workspace_id` on every row | One office, one sheet, many PCs. Scope is this PC’s SQLite, not a web workspace. |
| Roles `owner / operator / viewer` | T Books roles are `owner / admin / operator`. Map Loopbook `viewer` → cannot mutate. Do not rename. |
| `workspace-access.ts` invites | Hardcoded boss `thotanitin123@gmail.com` cannot be removed. Owner adds people on Access. |
| Live Google on every page open | Views never call Google. Auto-refresh is banned. |
| OT / Yjs / Automerge / last-write-wins on money | Two PCs must not silently overwrite. Submit uses fingerprint + rev (CAS). |
| Split voucher `20` into `20.1` | Column A is one integer row. Payments stay on that row (max 5). |
| Loopbook posting engine writing Postgres ledger as the source of truth | T Books source of truth for history is `Voucher_Raw_Data` + local SQLite. Hive Submit is one row. |
| Google Drive as the documents store | T Books stores a path/link on this PC. Drive ids are Loopbook-only. |
| Passwords in Google | Access tab is Name / Email / Role / Active only. Password hash stays on this PC. |
| Electron / Chrome wrapper of loopbook.com | T Books is Tauri. Not a browser of the website. |

If a file lives under Loopbook `src/lib/auth`, `src/lib/db.ts`, `src/routes/api/auth`, or uses `createServerFn` + Neon, **do not port the file**. Port only the **named function** from `src/lib/erp/*.ts` into `src/lib/t-books/business_rules/` **and** `src-tauri/src/core/business_rules/`.

---

## 2. What is already 1:1 (do not redo)

Nav labels, in this order: Board, Vouchers, Finance, Purchase, Sales, Vendors, Projects, Inventory, Logistics, HR, Documents, Duplicates, Explorer, Rules, Access, System.

Money: `calcPo`, `calcPayroll`, `calcSalarySlip`, `paymentStatus`, PUR-/PAY-/SAL- numbers, Indian FY trial, RBAC fail-closed for viewer.

Hive law: look = SQLite. Save = SQLite + dirty. Submit = one hive row, CAS. Refresh stops if this PC has dirty keys.

Windows extra (Loopbook does not have this): hive tabs Access, Voucher_Raw_Data, Purchase, Payments, Payroll, Sales_PO, Inventory, Logistics, Documents. System screen can inspect tabs, owner can create headers only, outbox can retry.

---

## 3. Still missing — allowed later (not a merge of Loopbook)

These are **T Books UI/hive work**, not “copy Loopbook screens”.

| Missing | Why it is not Loopbook-merge |
| --- | --- |
| Purchase merge / unmerge **pane** | Rules exist (`mergeBlockedReason`, `canUnmergeVoucher`). Desktop pane not built. Loopbook Data controls stay on the website. |
| Hide Submit until goods received | Rule `shouldPostPurchaseBill` exists. UI still lets Save locally. |
| Project money rollup like Loopbook project page | T Books project screen is local; do not import Loopbook server SQL. |
| Full posted ledger (Dr/Cr lines as Postgres) | Finance trial is built from local SQLite. Do not import `engine.server.ts` as-is. |
| Rules actually moving stock/ledger | Rules screen is local On/Off flags. Loopbook `rules` table posts into Postgres. Do not wire Neon. |
| Explorer Loopbook tables (`ledger_entries`, `accounts`, `parties`, …) | Those tables do not exist here. Explorer lists T Books SQLite tables only. |
| Documents as Google Drive files | Keep local path/url. |
| Multi-PC live share of a Save | Until Submit, PC A does not appear on PC B. That is v1 law, not a bug. |

---

## 4. How to port a rule (when you must)

1. Find the **named function** in Loopbook `src/lib/erp/`.
2. Copy behavior into **both** TS (`src/lib/t-books/business_rules/`) and Rust (`src-tauri/src/core/business_rules/`).
3. Same inputs → same outputs. Dummy ids in tests.
4. Do not copy the React page. Do not copy the server function. Do not add Neon.
5. Views still must not call Google.

---

## 5. Stop tests

- `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib`
- `npx tsc --noEmit`
- `node --experimental-strip-types --test tests/unit/modules.test.ts src/lib/t-books/business_rules/business_rules.test.ts`

If a change needs Loopbook login, Postgres, or Live Google on a list page: **reject it**.
