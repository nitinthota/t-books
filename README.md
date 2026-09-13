# T Books

Offline-first Windows ERP for a small office. Each person installs their own copy. The books live on **this PC**. The UI never calls Google — Refresh and Submit are manual, and only when online.

**Version:** 1.0.0

## Features

- **Offline-first** — open, edit, and save with no network
- **Google Sheets sync (controlled)** — Access (Loopbooks — Access), Refresh from read-only `Voucher_Raw_Data`, Submit to Voucher register
- **Multi-user safe** — one voucher row per submit; fingerprint conflict if another PC changed that row
- **Conflict detection** — dirty local vouchers block Refresh until Keep local / Discard local
- **Local-first performance** — SQLite on disk, lists virtualized, aimed at a 4 GB laptop

## Tech stack

- Tauri v2
- Rust (SQLite via rusqlite)
- React + CSS (no Electron, no Framer Motion)

## Setup

Office PCs run **only** `T-Books-Setup.exe`. The wizard can have several steps. If WebView2 is missing, Setup installs it as one of those steps (internet may be needed). You do not install Node, Rust, or WebView2 yourself. Books stay in `%LOCALAPPDATA%\T-Books`.

**Build** prerequisites (the PC that produces Setup.exe): Node 22, Rust 1.77+. **The installer is Windows-only** (NSIS). Build `T-Books-Setup.exe` on Windows 10/11 or from the GitHub Actions `windows-nsis` job. Linux cannot emit that `.exe`.

```bash
npm install
```

Desktop (Windows):

```bash
npm run tauri:dev
```

Typecheck / library tests:

```bash
npm run typecheck
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run test:books
```

Build the installer (`T-Books-Setup.exe`, per-user, Start menu + uninstall):

```bash
npm run tauri:build
```

Uninstall removes the app only. It does **not** delete `%LOCALAPPDATA%\T-Books`.

Place `credentials.json` (Google service account) in `%LOCALAPPDATA%\T-Books` on each PC that needs Access / Refresh / Submit. That file is never stored in this repository. See [docs/SHEET_HIVE_PLAN.md](docs/SHEET_HIVE_PLAN.md) and `secrets/google-service-account.example.json`. `Voucher_Raw_Data` is read-only; structured hive workbooks hold live rows.

## Folders

| Path | Role |
|---|---|
| `src-tauri/` | Rust backend, Tauri v2, SQLite, Google I/O |
| `src-tauri/src/core/business_rules/` | calcPo, status, payments, fingerprint — one place |
| `src-tauri/src/db.rs` | SQLite open, migrate, data folder |
| `src-tauri/src/sheets.rs` `access.rs` `vouchers.rs` `submit.rs` | Google sync (manual only) |
| `src/` | React UI |
| `src/lib/t-books/` | Dual-target TS (same rules as Rust, for preview) |
| `tests/` | Unit, integration, stress, write-back suites |
| `docs/` | Architecture, offline, sync, testing |
| `scripts/` | Build / test helpers |
| `public/` | Static assets (wasm, icons) |

Business rules live under `src-tauri/src/core/` (Rust) and `src/lib/t-books/business_rules/` (TypeScript). They are not fetched from GitHub at runtime.

## Security

- No real office rows, GST, bank details, or amounts in this repo
- No `credentials.json`, no SQLite files, no logs
- Passwords are hashed on this PC (Argon2id). Never written to Google
- Logs never record passwords, amounts, GST, or bank details
- Dummy tokens in tests only (`VOUCHER_1001`, `CUST_01`, `VEND_02`)

Owner sign-in on a new PC: `thotanitin123@gmail.com` sets the first local password.

See [docs/QUICK-START.md](docs/QUICK-START.md), [docs/architecture.md](docs/architecture.md), [docs/RELEASE.md](docs/RELEASE.md). Loopbook is rules-only — do not merge that repo: [docs/AI_DO_NOT_MERGE.md](docs/AI_DO_NOT_MERGE.md).
