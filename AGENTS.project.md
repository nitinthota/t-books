T Books — merged spec (rechecked)
Working name of the product: T Books.

GitHub web app nitinthota/loopbook is reference for business rules only. It is work in progress. Do not freeze its screens. Do not use Neon or Vercel for login or access.

1. What T Books is
Windows 10 and Windows 11 desktop app for a 3–5 person office.

Each person downloads and installs their own copy (T Books Setup.exe)
Each person has separate login (their email)
Fast on a 4 GB laptop
Premium, calm, polished desktop UI
Works offline; errors and offline state always shown
Google Sheet is the register + access list
Local SQLite on each PC is what the UI reads

Not a browser wrapper of the website. Not Electron unless RAM on 4 GB is proven acceptable. Prefer Tauri 2 + Rust + React + SQLite.

2. Hardcoded boss
Always allowed. Cannot be removed by a sheet row.

Email: thotanitin123@gmail.com
Role: owner
Only this email can add / disable people and change roles

If the Access tab is empty or missing, boss can still sign in and create the tab.

3. Two Google tabs only
Same spreadsheet. Restricted. App account (or boss Google) can read/write.
3.1 Voucher_Raw_Data — books register
Rules from git (voucher-data.ts, import-sheet.server.ts):

Column A = voucher number. Voucher 20 is voucher 20. Never 20.1
One sheet row = one voucher
Vendor, bank, A/c, IFSC, GST, project sit on that row
Up to 5 PI / payment blocks on the same row (PI no, date, value, % payment, description, TDS, paid, payment details, payment date, remaining, remarks)
Payments are added on the same voucher, not a new voucher
Status from amounts: Missing Tax Invoice, Pending, Partial, Advance, Full
Tax invoice NA needs a comment; otherwise flag it
Blank / odd cells must not crash (qty and money coerce to 0)
Dummy / sample rows (VOUCHER_*, SAMPLE%, CUST_*, VEND_*) are not real books

3.2 Access — who may open T Books























NameEmailRoleActiveNitinthotanitin123@gmail.comownerYes……admin / operatorYes / No

Sheet stores name, email, role, active only
No passwords in the sheet
Password is set on the PC, stored hashed locally
Boss adds a person by typing a row in Google or from T Books → Access (writes one row)
Active = No → blocked on next online login

Neon, Vercel, Better Auth, allowlist tables — not used.

4. What lives where



































DataWhereNotesWho can log inAccess tab + local cacheOnline check when possibleVoucher registerVoucher_Raw_Data + local copyRefresh pulls sheet → localEdits on this PCLocal SQLite firstImmediate saveSales PO, Purchase PO, HR, inventory, logistics, documentsLocal SQLite onlyNot in the voucher sheet; Refresh must not delete themPasswordsLocal hash on that PCNever printed, never put in Google

5. Network law (do not break)
Views never call Google.

Board, voucher open, vendor, project, search, reports, inventory, logistics, sales, HR = local only.








































ActionGoogle?WhatLook at anythingNoLocal SQLiteSave / add payment / new voucherNo (v1)Local SQLite + dirty flagRefreshYesPull whole Voucher_Raw_Data into local registerLogin while onlineYesRead Access onlyAccess add/disable (boss)YesWrite one Access rowWrite-back to voucher sheetNot in v1Planned later
If Google fails: keep local data, show error popup. Never wipe the screen to empty.

6. Offline
App must open without internet.

Banner: Offline — working on this PC. Access list and Refresh paused.
All edits save locally
If Google / Access check fails: error popup with the real message
Last cached Access list is used
Boss can always enter on a PC that already has T Books installed


7. Login and access flow

Install T Books on the PC
Sign in with email + password
Online: read Access
Email not listed or Active ≠ Yes → deny (except hardcoded boss)
Listed → allow, refresh local Access cache

Offline: allow only if this email was accepted on this PC before and is still in the cache
First login on a PC for that email → set password (hashed locally)
Temp / first password cannot be reused as the long-term one if you later add “must change”
Separate access: no shared office password

Roles:

owner (boss only in practice): Access, Refresh, all modules
admin: edit books, Refresh, no owner transfer
operator: edit books, no Access screen


8. Multi-PC (v1 truth)
Same app, many PCs, each install is independent.
Until write-back exists:

A payment saved on PC A does not appear on PC B
PC B only sees the sheet after Refresh
Two people may edit the same voucher on two PCs; they will diverge

Locked default until you change it: every PC may edit; other PCs will not see it yet. Refresh on a PC that has dirty local vouchers must stop and list those voucher numbers — it must not silently overwrite local work with the sheet.
When write-back is added later:

Submit of voucher 20 reads only that sheet row
Fingerprint matches → write that row
Fingerprint differs → do not write. Message: Voucher 20 was just updated by another user. Reload and submit again.
Different voucher numbers never block each other
New number already used → reject


9. Refresh (v1 sync)
Button on Board: Refresh from Google.

Read Voucher_Raw_Data once
If this PC has dirty local vouchers → stop, list them, user chooses Keep local / Discard local per voucher
Replace local register only: vouchers, lines, vendors-from-sheet, projects-from-sheet
Do not delete: po_master, po_items, hr_payroll, inventory, logistics, documents, local users
Show counts + errors (never hide errors as “0 vouchers”)
One voucher per column A


10. Modules (logic from git, UI rebuilt for desktop)
Port rules from:

src/lib/erp/voucher-data.ts
src/lib/erp/money.ts (paise in store, INR on screen)
src/lib/erp/engine.server.ts
src/lib/erp/adapt.server.ts
src/lib/erp/import-sheet.server.ts
src/lib/erp/po-calc.ts + po-hr-api.ts
src/lib/erp/rbac.ts
migrations/0016_po_hr.sql

Screens:

Sign in / set password
Board — counts, Refresh, errors
Vouchers — filters (missing tax inv, NA needs comment, pending, advance, full)
Open voucher — value, paid, still to pay; add payment on same voucher; description visible
Vendors + vendor detail (vouchers + projects)
Projects + project detail (vendors on job, cost vs POs received)
Sales — contract PO (items, GST, totals) + project POs
Purchase — contract PO + simple project PO
HR — people + payroll (PF company, PF employee, PF total, TDS)
Inventory — cost, project optional, type, size, qty; edit / move to project / delete
Logistics — project, vehicle no, invoice, start / reach dates; edit / move / delete
Access — boss only
Search on vendors, projects, inventory, logistics

PO math stays one function for preview and save (calcPo). Negative PO values allowed (deductions). No duplicate PO number on the same project.

11. UI / motion / premium

Desktop chrome: left nav, main pane, not a phone layout
Product name in UI: T Books
Back top-left on every page
Unsaved: Save & go back / Discard / Cancel
Sections: Basic / Contract / Items / Summary
Inline validation; live totals
Failed load = red banner with the real message, never a fake empty list
Motion only: 150–220ms fade + 8px slide-up, press 0.96, input focus ring
No Framer Motion, no blur on long lists, honor reduced motion
Virtualize long voucher lists
Quiet paper + ink palette (web tokens: warm paper #f6f1e8, ink #1c1915, navy #243e5a, gold accent spare)


12. Performance budget (4 GB laptop)

Installer as small as Tauri allows (target well under Electron)
Per-user install so PCs without admin can still install if possible
Cold start under ~3s
Board from local DB under ~1.5s
Refresh may be slow (Google) — progress text, UI not frozen
No polling the sheet
Idle RAM far below a Chrome window


13. Installer

Name: T Books
File: T-Books-Setup.exe
Windows 10 and 11
Proper installed app (Start menu + uninstall), not a raw portable zip for v1
Data folder under the user profile (not Program Files)
GitHub repo changes do not auto-rebuild the exe; new setup.exe when you ship a version


14. Security and non-disclosure

Do not put real vendor names, phones, GST, bank, amounts, or raw sheet rows in code, logs, or README
Examples only: CUST_01, VEND_02, VOUCHER_1001
Do not log passwords or service-account JSON
Explorer-style raw tables must hide password / token / secret columns
Sheet stays Restricted; team uses T Books, not five people editing the same Google row by hand


15. Explicitly removed

Neon as login/access
Vercel as login/access
Google Sign-In as the app login
Auto refresh on every page open
Splitting one column-A voucher into many
Silent overwrite of a voucher another user just changed (when write-back exists)
Wiping PO / HR / inventory on Refresh


16. Build order

Installer + sign in + local SQLite + hardcoded boss
Access tab read/write + offline cache + banners
Board + vouchers + vendor + project from local store
Refresh from Voucher_Raw_Data with dirty-voucher guard
Sales / Purchase / HR / inventory / logistics
Polish motion and 4 GB pass
Later: write-back + per-voucher fingerprint


Locked defaults (you did not pick; change if wrong)

Until write-back: all PCs may edit; they do not share edits yet
Refresh vs local dirty: stop and list dirty voucher numbers
Access sheet: name + email + role + active; passwords only on the PC
Non-boss may type in the Access tab in Google, but the app treats only boss as able to grant from inside T Books
Branding: T Books only, monogram T if no logo
Updates: new T-Books-Setup.exe, no auto-updater in v1

That is the full merge. If any locked default is wrong, say the number.

This conversation belongs to a Grok project. The project's files are mounted at `/workspace/artifacts` — look there for user-provided sources before concluding the workspace has no project files. Files written there persist to the project across conversations.