# T Books release checklist (v1.0.0)

Installer file: **T-Books-Setup.exe** (per-user, Start menu, uninstall).

This is a **Windows 10/11 desktop** app (Tauri 2 + Rust + React + SQLite). It is not a Vercel / browser site. There is **no auto-updater** in v1: ship a new `T-Books-Setup.exe` when you release.

## Before shipping

1. Fresh install opens, owner can set a password.
2. Upgrade install keeps `%LOCALAPPDATA%\T-Books` (books, backups, logs).
3. Uninstall does **not** delete that data folder.
4. Settings → Backup, then Restore (confirm) round-trips vouchers.
5. Offline: board, modules, and local edits work. Banner is exact.
6. Refresh from `Voucher_Raw_Data` (manual) and Access refresh work when online.
7. Submit fingerprint conflict: second PC cannot overwrite the first PC’s sheet row.
8. Logs never contain passwords, amounts, GST, or bank details.
9. 4 GB RAM laptop: cold start under 3s, lists stay usable.
10. The installer does **not** contain `credentials.json`, real books, or service-account PEM.

Distribution: direct `.exe` (GitHub Release or office share). No store listing required.

## Build (Windows PC — this is what produces Setup.exe)

Need: Windows 10/11, Node 22, Rust stable (`rustup`), WebView2 (Win 11 already has it). NSIS is pulled by the Tauri bundler; you do not install Electron.

```
npm ci
npm run tauri:build
```

That runs the Tauri frontend build, compiles the Rust app with the `desktop` feature, builds a **current-user** NSIS installer, then copies it to:

`dist/installers/T-Books-Setup.exe`

Do **not** run `npx vite` or `cargo tauri` without the npm script. `npm run tauri:build` is the supported command (`tauri build` + rename).

Linux / this sandbox **cannot** emit a Windows NSIS `.exe` (no Windows target, no NSIS). Use a Windows machine or the GitHub Actions job `windows-nsis` on `windows-latest`, then download the `T-Books-Setup` artifact.

## After install (each PC)

1. Run `T-Books-Setup.exe` (no admin needed for a per-user install).
2. Open **T Books** from the Start menu folder **T Books**.
3. Sign in as `thotanitin123@gmail.com` and set the password for **this PC**.
4. If Access / Refresh / Submit are needed, copy `credentials.json` into `%LOCALAPPDATA%\T-Books` **after** install. Never bake that file into git or the installer.

Uninstall (Settings → Apps, or the Start menu uninstall entry) removes the program only. Books stay under `%LOCALAPPDATA%\T-Books`.
