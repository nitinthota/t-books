# Architecture

T Books is a **desktop** app. Each PC has its own install and its own SQLite file.

```
PC
 ├─ UI (React)  ──reads──►  SQLite  %LOCALAPPDATA%\T-Books\tbooks.db
 └─ Rust        ──writes─►  SQLite
                  │
                  └── only on explicit commands, if online:
                        Refresh Access (Loopbooks — Access)
                        Refresh Voucher_Raw_Data (read-only archive)
                        Submit one voucher → Voucher register
                        Check for updates
```

## Layers

1. **UI** — `src/components/t-books/`  
   Screens read the local store. They never call Google.

2. **Commands** — `src-tauri/src/lib.rs` (Tauri IPC)  
   Login, refresh, submit, office CRUD, backup/restore.

3. **Business rules** — `src-tauri/src/core/business_rules/`  
   calcPo, payments (max 5 on the same voucher), status, fingerprint.  
   TypeScript twin: `src/lib/t-books/business_rules/` (preview only).  
   Same inputs → same outputs. Loopbook is a reference, not a runtime dependency.

4. **SQLite** — `src-tauri/src/db.rs`  
   Schema version in `app_meta`. Additive migrations. WAL.

## Google

Service account JWT from `credentials.json` on this PC (or `TBOOKS_GOOGLE_SA_JSON` / gitignored `secrets/google-service-account.json`). Views never call Google.

- **Read-only archive:** `Voucher_Raw_Data` on the source spreadsheet. Never written.
- **Live hive:** Loopbooks Drive workbooks in `src-tauri/hive-map.json` (Access, Voucher register, Payments, Purchase, …).
- Access write: owner, one row. Passwords hashed locally; sheet has Name / Email / Role / Active / Account_Type.

See [SHEET_HIVE_PLAN.md](SHEET_HIVE_PLAN.md).

## What lives where

| Data | Where | Refresh |
|---|---|---|
| Who can sign in | Access tab + `access_cache` | Manual |
| Voucher register | `Voucher_Raw_Data` (history, read-only) + `Loopbooks — Voucher register` + local `vouchers` | Refresh reads archive; Submit writes register |
| Sales / purchase / HR / inventory / logistics / documents | Local tables only | Never deleted |
| Passwords | `users_local.password_hash` | Never in Google |

## Roles

- **owner** (`thotanitin123@gmail.com`) — Access, Refresh, Submit, all modules
- **admin** — edit, Refresh, Submit; no Access screen
- **operator** — edit only

## Version

`app_meta.version` = `1.0.0` (also `Cargo.toml`, `tauri.conf.json`, UI Settings).
