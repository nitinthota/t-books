# T Books — simplified requirements (owner lock)

Working name: **T Books**

This replaces the hive / CAS / dirty-voucher model for day-to-day use. Loopbook remains rules reference only. Desktop app stays Tauri + SQLite; Google Sheets is a plain register, not a multi-PC fingerprint system.

## 1. Sync model — remove hive complexity

**Remove**

- Hive kinds, fingerprint CAS, Submit conflict flows
- Dirty-voucher blocking of Refresh
- Keep Local / Discard Local & Refresh dialog and buttons

**Replace with simple Sheets I/O**

| Action | Behaviour |
| --- | --- |
| Read | Pull from Google Sheets into local SQLite (Refresh / load) |
| Write | Push this PC’s row/data to Google Sheets (Save / Save to Google) |
| Offline | App still opens; local SQLite works; Google read/write paused with a clear error |
| Views | Still read local SQLite only (no Google on every screen open) |

No silent wipe of the UI when Google fails. Show the real error.

**Remove drafts**

- Clear **Remove drafts** control that works.
- Clears local-only draft / unsaved local rows.

## 2. Sales module

- Remove Contract PO totally.
- One create action only: **New PO**.
- Single editor: project, PO number, client, GST, terms, items, summary.
- No kind toggles, no clone.

## 3. Purchase / New bill

- One create action only: **New bill**.
- No contract vs simple toggle layer.
- One form: project, vendor, PUR number, items/total, tax invoice, payments.

## 4. Project expense

- **Paid by** — employee only.
- Same fields as Techsol purchases (until listed: T Books purchase bill fields + Paid by employee).

## 5. What stays

- Windows desktop, per-user install, offline-first UI.
- Login email + local password; Access; owner `thotanitin123@gmail.com`.
- UI reads SQLite; offline banner; no Neon/Vercel/Google Sign-In as app login.
- Dummy data only in code. Installer `T-Books-Setup.exe` when shipped, not committed every push.

## Build order

1. Sales single New PO — done in UI
2. Purchase single New bill — done in UI
3. Remove drafts that works
4. Drop Keep/Discard refresh dialog; simple Refresh
5. Simple write path to Google
6. Project expense + Paid by employee
7. Polish + installer
