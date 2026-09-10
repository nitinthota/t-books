# T Books release checklist (v1.0.0)

Installer file: **T-Books-Setup.exe** (per-user, Start menu, uninstall).

Before shipping:

1. Fresh install opens, owner can set a password.
2. Upgrade install keeps `%LOCALAPPDATA%\T-Books` (books, backups, logs).
3. Uninstall does **not** delete that data folder.
4. Settings → Backup, then Restore (confirm) round-trips vouchers.
5. Offline: board, modules, and local edits work. Banner is exact.
6. Refresh from `Voucher_Raw_Data` (manual) and Access refresh work when online.
7. Submit fingerprint conflict: second PC cannot overwrite the first PC’s sheet row.
8. Logs never contain passwords, amounts, GST, or bank details.
9. 4 GB RAM laptop: cold start under 3s, lists stay usable.

Distribution: direct `.exe` / website download. No store listing required.

Build (maintainer):

```
npm run build:tauri
cargo tauri build
```

Rename the NSIS output to `T-Books-Setup.exe` if the bundler uses a versioned name (`scripts/rename-installer.mjs`).
