# T Books — final Sheets hive plan

**Status:** plan only. Do not merge Loopbook. Do not commit secrets. Dummy ids only: `CUST_01`, `VEND_02`, `VOUCHER_1001`, `PUR-0001`, `PAY-0001`, `PO-1@CUST_01`, `INV-1`, `SAL-0001`.

This is the implementable contract for a later coding agent. Google Sheets is the **office register** because there is no server. Local SQLite is the **UI cache**. The UI never calls Google.

---

## Locked product facts (already in the app)

| Fact | Where today |
|---|---|
| No Google user login | `auth.rs` — email + local Argon2id hash |
| Service account JWT, Sheets API only | `sheets.rs` scope `https://www.googleapis.com/auth/spreadsheets` |
| Credentials file | `%LOCALAPPDATA%\T-Books\credentials.json` (`CREDENTIALS_FILE_NAME`) |
| Spreadsheet id | `credentials.json` field `spreadsheet_id` / `spreadsheetId` / `sheet_id`, else `DEFAULT_SPREADSHEET_ID` in `lib.rs` |
| Hardcoded owner | `thotanitin123@gmail.com` — always allowed; cannot be removed by a sheet row |
| Access tab | Name / Email / Role / Active — **no password column** (`ACCESS_HEADERS`) |
| Hive law | Look = SQLite. Save = SQLite + dirty. Submit = one hive row, CAS (`fp` + `rev`). Refresh = pull tabs, stop if this PC has dirty keys |
| Bootstrap | Owner may create **missing tabs with headers only** (`create_tab_with_headers`, `bootstrap_hive_tab`) |
| Offline | App opens; banner `Offline — working on this PC. Access list and Refresh paused.` Missing credentials: `Google credentials not found. Running offline.` |

---

## 1. Auth / access model

### 1.1 Service account vs Google login

**There is no Google Sign-In in T Books.** People never click “Sign in with Google.”

- **App login:** the person’s office email + a password stored **on that PC** (`users_local.password_hash`, Argon2id).
- **Google identity:** one **Google Cloud service account** (`client_email` like `t-books@PROJECT.iam.gserviceaccount.com`). The app signs a JWT and exchanges it at `https://oauth2.googleapis.com/token`. That token is used for **all** sheet read/write.
- The same service account is Editor on the **one** Restricted spreadsheet. It is the only Google principal the app uses.

Do not add OAuth consent screens, user tokens, Neon, Vercel, or Better Auth.

### 1.2 Access tab — recommended columns (locked)

Row 1 frozen. App maps by **header name**, not column letter.

| Header | Meaning | Example |
|---|---|---|
| Name | Display name | `Nitin` |
| Email | Login id (lowercase) | `thotanitin123@gmail.com` |
| Role | `owner` / `admin` / `operator` | `owner` |
| Active | `Yes` / `No` | `Yes` |

Optional later (do **not** add unless the owner asks in a follow-up): `Notes`.

**Do not add:** `Password`, `PasswordHash`, `Type of account` as a secret, PIN, OTP.

“Type of account” in the user’s request maps to **Role**, not a Google account type.

### 1.3 Security conflict: passwords in the sheet

The latest request said the Access sheet should hold name, type of account, **password**, etc.

That **conflicts** with the locked spec (`AGENTS.project.md` §3.2, `AI_DO_NOT_MERGE.md`: “Passwords in Google — never”) and with current code (`parse_access_values` ignores any password column; `passwords.rs` hashes locally).

**Do not store plaintext passwords in Google Sheets.** Anyone who can open the Restricted spreadsheet (or a copy, export, version history, “anyone with the link” mistake, or a screenshot) sees **every office password**. Combined with `credentials.json` on a laptop, an attacker can:

1. Read/write **all** hive tabs (vouchers, bank, GST, payroll).
2. Change Access rows (disable the owner, add themselves).
3. Impersonate every person if passwords live in the sheet.

That is full office takeover with **one JSON file + one spreadsheet**.

**Recommended (keep current product):**

- Sheet = **allowlist** only (Name, Email, Role, Active).
- Password = set on first login **on that PC**, Argon2id in `users_local`.
- Consequence: the same person on PC B sets **their own** first password on PC B. There is no shared office password. That is intentional for a 3–5 person shop with no server.

**If they insist on “one password that works on every PC” without a server:**

| Option | Verdict |
|---|---|
| Plaintext `Password` column | **Forbidden.** Never implement. |
| `PasswordHash` column (Argon2id PHC string) written by T Books only | Weak: hashes leak; anyone with sheet Editor can **replace** a hash and take that login; Google version history retains old hashes. Use only if they accept that risk in writing. |
| Keep hash local (current) | **Preferred.** |

If hash-in-sheet is forced later:

- App **writes** the hash on first-set / change-password (owner/admin policy). Humans never type a hash by hand.
- App **never** displays the hash (Explorer already hides `password` columns).
- Login online: verify against sheet hash **or** local hash, then refresh local.
- Login offline: local hash only.
- Still never log `private_key` or hashes.

### 1.4 First run — `thotanitin123@gmail.com`

Keep this exact flow:

1. Install T Books. SQLite is created under `%LOCALAPPDATA%\T-Books`.
2. Owner types `thotanitin123@gmail.com`. `is_hardcoded_owner` returns true even if Access is empty, missing, or Active=No.
3. First time on that PC: **Set password** (`set_first_password`). Hash stays in `users_local`.
4. If credentials + network exist: login may prefetch Access. If Access tab is missing, **owner** may bootstrap headers-only and later add people.
5. Other emails: denied unless Access cache (or a fresh Access pull) lists them with Active=Yes and a mutating role (`owner`/`admin`/`operator`). `viewer` cannot open T Books.
6. Offline: owner can always enter on a PC that already has T Books. Others only if they were accepted on this PC before (cached Access).

Boss adds people from T Books → Access (writes **one** row) or by typing a row in Google. Only the app treats boss as able to grant from inside T Books.

---

## 2. Secure `credentials.json` on Windows

### 2.1 What the file is

A **Google Cloud service account key** (standard Google JSON) plus the office spreadsheet id.

Required fields the app already parses (`parse_service_account`):

| Field | Required | Notes |
|---|---|---|
| `type` | expected `service_account` | Informational |
| `client_email` | **yes** | JWT `iss` |
| `private_key` | **yes** | RSA PEM; `\n` escaped in JSON |
| `private_key_id` | optional | JWT `kid` |
| `token_uri` | optional | default `https://oauth2.googleapis.com/token` |
| `spreadsheet_id` | strongly recommended | else hardcoded `DEFAULT_SPREADSHEET_ID` |

Optional Google fields (`project_id`, `client_id`, `auth_uri`, `auth_provider_x509_cert_url`) may be present; ignore them.

**APIs to enable on that Cloud project:** Google Sheets API. Drive API is **not** required for tab bootstrap or cell I/O. Drive API is only needed if a later phase implements “create a brand-new spreadsheet in the owner’s Drive” — see §4.3.

**OAuth scope (keep narrow):** `https://www.googleapis.com/auth/spreadsheets` (read/write cells + addSheet). Do **not** request `drive` or `spreadsheets.readonly`.

Example shape (fake key — never a real PEM in git):

```json
{
  "type": "service_account",
  "client_email": "t-books@example-project.iam.gserviceaccount.com",
  "private_key_id": "abc123",
  "private_key": "-----BEGIN PRIVATE KEY-----\nMIIE...\n-----END PRIVATE KEY-----\n",
  "token_uri": "https://oauth2.googleapis.com/token",
  "spreadsheet_id": "PASTE_THE_OFFICE_SHEET_ID_HERE"
}
```

### 2.2 Where it lives

**Canonical path (already product):** `%LOCALAPPDATA%\T-Books\credentials.json`

That folder already holds `tbooks.db`, `logs/`, `backups/`. Uninstall does **not** delete it.

**Do not:**

- Commit the file (already in `.gitignore`: `credentials.json`, `**/credentials.json`, `*.pem`, `*.p12`, `*.key`).
- Bundle a real key inside `T-Books-Setup.exe`.
- Store it in Program Files, OneDrive-shared folders, or the git working tree.
- Log `private_key`, `private_key_id` in full, Bearer tokens, or the JSON body (`hive.rs` already redacts `private_key` / `password` / `bearer` in outbox errors; keep that).

### 2.3 Hardening to add (implementation phase — not this PR)

1. **Installer:** ship empty data folder only. First-run System screen: “Place credentials.json here” with the full path. Optional **Import…** copies a user-selected JSON into that path after validating `client_email` + `private_key` (never echo the key).
2. **NTFS ACL:** on first write, restrict `%LOCALAPPDATA%\T-Books` to the Windows user (no `Everyone`). Document: do not share this folder.
3. **DPAPI at rest (recommended):** after a successful parse, rewrite the file as an opaque blob via `CryptProtectData` (user scope) so a copied file is useless on another Windows account. Keep a clear-text import path for first drop. If decrypt fails, show a real error; do not wipe SQLite.
4. **Distribution:** boss creates **one** key in Google Cloud, downloads JSON **once**, puts `spreadsheet_id` in it, then copies via USB / Signal / encrypted zip — **not** email to a group, not WhatsApp, not the public repo. Each office PC gets the **same** file (same SA + same sheet).
5. **Rotation:** create a new SA key, replace the file on every PC, disable the old key in Cloud Console. App caches JWT ~1 hour (`sheets.rs`); restart T Books after rotation.
6. **Missing file:** app **must** still open. Local SQLite is the books on this PC. Access/Refresh/Submit show the existing credentials banner. Never delete local rows because Google is missing.

### 2.4 Repo / docs hygiene (already mostly done)

- `.gitignore` covers credentials.
- README already says the file is never in the repository.
- Tests use fake JSON with a stub PEM (`sheets.rs` unit tests). Never paste a live key.
- `DEFAULT_SPREADSHEET_ID` in source is an id, not a secret, but new offices should always override via JSON so a leaked repo id is harmless if the sheet stays Restricted and unshared.

---

## 3. Spreadsheet layout (huge data, still human-readable)

### 3.1 One spreadsheet, many tabs

**Recommend one Restricted spreadsheet** for a 3–5 person office (current product). Reasons:

- One share to the service account.
- One id in `credentials.json`.
- Humans open one URL to audit.
- Sheets cell cap is **per spreadsheet** (~10 million cells). A small office will not hit that if line items live on **child tabs**, not 200 extra columns on the parent.

**Do not** create a new spreadsheet per module or per session. That would scatter ids, shares, and backups.

Optional later (only if cell count actually hurts): archive FY into a second spreadsheet and keep the live year in the hive. Not v1.

### 3.2 What “create new Google sheets” means

**Owner bootstrap of missing tabs (headers only)** inside the **existing** spreadsheet.

Not: `spreadsheets.create` on every launch. Not: a new file per Submit.

Boss (human) still creates the empty spreadsheet in Google (Restricted), shares it with the service account **as Editor**, pastes the id into `credentials.json`. The app then adds tabs `Access`, `Voucher_Raw_Data`, … when the owner runs System → bootstrap.

Optional one-shot later: Sheets API `spreadsheets.create` as the SA, then the boss **must** add their Gmail as owner in Drive. Prefer the human-created file so the office owns the Drive object.

### 3.3 Tab catalog (final hive)

Frozen **row 1** = human headers. Data from row 2. Dummy rows (`VOUCHER_*`, `SAMPLE%`, `CUST_*`, `VEND_*`) are not real books.

Every mutating tab except historical voucher layout includes **`fp`** (SHA-256 hex) and **`rev`** (integer) for CAS. Access does not need fp/rev (tiny table; owner writes one row).

| Tab | Hive kind | Unique key | Parent vs child | Status today |
|---|---|---|---|---|
| `Access` | `access` | Email | — | Read/write + bootstrap exist |
| `Voucher_Raw_Data` | `voucher` | Integer column A (`1001` not `20.1`) | **One row = one voucher**; 5 payment slots **on the same row** | Refresh + Submit exist. Header list in code is **incomplete** vs historical 5×11 payment columns — map by header / fixed historical offsets |
| `Purchase` | `purchase` | `PUR-0001` | Parent bill | Submit cells exist; **items not on sheet** |
| `PO_Items` | `po_item` (new kind) | `PUR-0001#01` | Child of Purchase **or** Sales (`parent_key`) | **Does not exist** — add this so 200 lines do not explode the parent |
| `Payments` | `payment` | `PAY-0001` | Child of purchase by `po_number` | Submit exists |
| `Sales_PO` | `sales_po` | `PO-1@CUST_01` (`po_number@project`) | Parent | Submit exists; items missing |
| `Payroll` | `salary` | `SAL-0001` | — | Submit exists |
| `Inventory` | `inventory` | `INV-1` | — | Submit exists |
| `Logistics` | `logistics` | `TRIP-1` | — | Submit exists |
| `Documents` | `document` | `DOC-1` | Path/URL on PC, not Drive file ids | Submit exists |
| `Vendors` (optional master) | `vendor` | `VEND_02` | Allowlist names/GST/bank for humans | Not in hive yet; can stay derived from voucher/purchase rows |
| `Projects` (optional master) | `project` | `CUST_01` | Same | Optional |

Rename note: code today names the purchase tab `Purchase`. The user asked for “Purchase bills”. **Keep tab id `Purchase`** (already in `tab_name`) so existing PCs do not miss the tab. Header row can say `po_number` / `Bill` in comments. Do not invent a second tab `Purchase bills`.

**Do not invent voucher `20.1`.** Payments on history stay on the voucher row (max 5 slots × 11 fields). A future `Voucher_Payments` child tab is **out of scope** unless the office outgrows 5 slots; that would be a new module, not a rewrite of column A.

### 3.4 `Voucher_Raw_Data` columns (humans + app)

Historical layout (keep compatible with Loopbook import rules / `vouchers.rs`):

- Columns 0–8 (A–I): `serial_no`, `voucher_date`, `tax_inv_no_date`, `vendor_name`, `bank_name`, `account_no`, `ifsc`, `gst_no`, `project_name`
- Then **five** identical payment blocks, each 11 cells: PI no, date, value, % payment, description, TDS, paid, payment details, payment date, remaining, remarks
- Append `fp`, `rev` at the **end** when bootstrapping **new** tabs so CAS is visible to humans. When reading an old sheet without those headers, compute fingerprint from payment fields as today and treat missing `rev` as 0.

Parser rule: **match header names** when row 1 looks like a header; else fall back to the historical offset layout (current `HEADER_COLS` + `SLOT_WIDTH`). Never require humans to keep column letters stable if they insert a Notes column — new columns must be named in row 1.

### 3.5 Parent + child for huge line items

**Problem:** one PO with 200 items as extra columns on `Purchase` would make the parent tab unreadable and waste cells (200 × ~8 fields × thousands of POs).

**Rule:**

- `Purchase` / `Sales_PO` row = header money + vendor/client + GST + goods received + tax invoice + `fp`/`rev` of the **parent fields only**.
- `PO_Items` one row per line:

| Header | Example |
|---|---|
| parent_key | `PUR-0001` or `PO-1@CUST_01` |
| parent_kind | `purchase` / `sales_po` |
| line_no | `1` |
| item_name | `Cable` |
| qty | `10` |
| rate | `100` |
| gst_pct | `18` |
| amount | `1180` |
| fp | hash of this line |
| rev | integer |

Unique key: `parent_key#line_no` (`PUR-0001#01`).

**Submit of a PO (one user click, still “one hive transaction” from the user’s view):**

1. CAS the parent row (`Purchase` or `Sales_PO`).
2. If parent CAS succeeds, replace **that parent’s** item rows only (delete/rewrite lines with that `parent_key` in `PO_Items`, or CAS each line). Never rewrite other POs’ items.
3. If Google fails after parent write: keep parent dirty + outbox; do not clear local dirty; System → retry.

Refresh: pull parent tab + `PO_Items` filtered in memory by `parent_key` into `purchase_po_items` / `sales_po_items`. Dirty guard: if this PC has dirty `PUR-0001`, skip that key (same Keep local / Discard local UX).

### 3.6 Cell budget (order-of-magnitude)

Sheets ~10M cells / spreadsheet.

| Tab | Rough cells | 3–5 person office |
|---|---|---|
| Access | 5 people × 4 cols | tiny |
| Voucher_Raw_Data | 5 000 vouchers × ~66 cols | ~0.3M |
| Purchase + Payments | thousands × ~12 | small |
| PO_Items | 500 POs × 50 lines × 10 cols | ~0.25M |
| Other masters | small | small |

A 200-item PO costs **200 rows on `PO_Items`**, not 200 columns on `Purchase`. That is the scale design.

If a tab approaches millions of rows: archive a FY copy, do not widen rows.

### 3.7 Human readability rules

1. Freeze row 1; bold headers; wrap off on key columns.
2. Human names in row 1 (`vendor_name`, not `C`).
3. App binds `find_col` / header map (Access already does this).
4. Restricted sharing: office uses T Books; do not have five people edit the same Google row by hand (CAS will fight them).
5. No real vendor/GST/bank in git, logs, or README. Dummies only.

---

## 4. App architecture without a server

```
Each Windows PC
  UI (React)  ──never Google──►  SQLite  %LOCALAPPDATA%\T-Books\tbooks.db
  Rust IPC
       Save     → SQLite + is_dirty
       Submit   → service account → one hive row (CAS fp+rev) + children if PO
       Refresh  → service account → pull tab(s) → upsert SQLite (dirty guard)
       Login    → Access tab when online; else cache; owner always
```

**Mandatory for 4 GB laptops:** Board, lists, editors, search, trial, Explorer read **SQLite only**. No polling. Refresh may be slow — progress text, UI not frozen.

**Service account does all sheet I/O.** Views must not grow new Google calls (see `PARITY_MATRIX` hive command table).

**Multi-PC:** until Submit, PC A does not appear on PC B. Two people on the same key: second Submit gets `{KEY} was just updated by another user. Reload and submit again.` Different keys never block each other.

**Refresh vs dirty:** stop and list dirty keys; Keep local / Discard local. Never silent overwrite. Never delete sales/purchase/HR/inventory/logistics/documents because voucher Refresh ran (extend Refresh later to **opt-in per module** or a “Refresh all hive tabs” that still respects dirty keys).

**Outbox:** `pending_submit` already retries one row. Keep it.

---

## 5. What exists vs what to add

### Already built

- Tauri + SQLite schema, local office modules, voucher 5-slot payments
- Service account load/JWT/Sheets GET PUT append / addSheet
- Access allowlist + owner bootstrap + write one row
- Voucher Refresh + dirty guard + Submit CAS
- Office Submit for Purchase, Payments, Payroll, Sales_PO, Inventory, Logistics, Documents (parent cells only)
- System `hive_status` + `bootstrap_hive_tab`
- `.gitignore`, credentials path, secret column hiding
- Business rules twins (money, calcPo, paymentStatus, RBAC) — Loopbook is reference only

### Gaps this plan fills (later PRs)

1. Treat **all** office tables as hive-backed (Refresh per tab), not “vouchers in Google, everything else local-only forever.”
2. Add **`PO_Items`** (and wire Submit/Refresh) so line items survive across PCs.
3. Optional Vendors / Projects master tabs.
4. Complete `Voucher_Raw_Data` bootstrap headers to include 5 payment blocks + `fp`/`rev`.
5. Credentials: Import UI, NTFS ACL, optional DPAPI; never bundle keys.
6. Header-name mapping on every tab (not only Access).
7. Hide Submit on purchase until goods received (`shouldPostPurchaseBill`) — UI gap, not hive.
8. Purchase merge/unmerge pane — UI gap.

---

## 6. Phased build order (later agent)

Execute in order. Each phase: `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib`, `npx tsc --noEmit`, books unit tests. Dummy ids only. No Loopbook merge. No live secrets in CI.

### Phase 0 — Contract freeze (this document)

Owner confirms: **no plaintext passwords in Sheets**; one Restricted spreadsheet; SA JSON on each PC.

### Phase 1 — Credentials hygiene

- System screen: path, Import, “missing → offline SQLite still works.”
- Validate JSON; never log PEM.
- Optional DPAPI + ACL.
- Docs: USB distribution + key rotation (`docs/QUICK-START.md` / `RELEASE.md` pointers).

### Phase 2 — Tab headers humans can read

- Expand `tab_headers(KIND_VOUCHER)` to historical payment slot names + `fp`/`rev`.
- Humanize other `tab_headers` if needed (`Pay number` vs `pay_number` — pick one set and map case-insensitively).
- Bootstrap missing tabs only; freeze row 1 via Sheets `repeatCell` / `updateSheetProperties` if cheap, else document “owner freeze row 1 in Google.”

### Phase 3 — `PO_Items` hive

- New kind + tab name `PO_Items`.
- SQLite already has `purchase_po_items` / `sales_po_items` — add `hive_rev` / `source_hash` / `is_dirty` if missing on lines, or dirty only on parent.
- Submit parent then children. Refresh both. Tests: 200 dummy lines on `PUR-0001` do not widen `Purchase`.

### Phase 4 — Refresh all hive tabs

- Board/System: Refresh Access, Refresh vouchers, Refresh office (or one “Refresh from Google” that pulls every **existing** tab, skips missing tabs without wiping local).
- Dirty guard per key/kind (`list_dirty_keys` already exists).
- Missing tab → keep local, message `{tab} is not on the sheet. Local data was not deleted.`

### Phase 5 — Masters (optional)

- `Vendors` / `Projects` tabs if the office wants a human directory. Else keep deriving from register rows.

### Phase 6 — Polish

- Hide Submit until goods received; merge pane; 4 GB pass; never auto-refresh on page open.

---

## 7. Explicit non-goals

- Neon / Vercel / Better Auth / Google Sign-In
- Merging the Loopbook repo
- Splitting voucher `20` into `20.1`
- Last-write-wins / OT / Yjs on money
- Google Drive as the documents store
- Bundling a real service-account key in the installer
- Creating a new spreadsheet on every session
- Putting plaintext passwords in Access

---

## 8. Coding-agent checklist (copy into the first implementation PR)

- [ ] Views still do not call Google
- [ ] Save = SQLite + dirty; Submit = CAS; Refresh = pull + dirty guard
- [ ] Access headers remain Name, Email, Role, Active unless a **written** exception for hash-only
- [ ] `credentials.json` only under `%LOCALAPPDATA%\T-Books`, gitignored
- [ ] Same SA for read and write; Sheets API only
- [ ] Bootstrap = headers on missing **tabs**, one spreadsheet id
- [ ] `PO_Items` for line items; vouchers stay one integer row with 5 slots
- [ ] Dummy data in tests; no real office rows in logs
- [ ] Owner `thotanitin123@gmail.com` still cannot be locked out by the sheet
