# T Books release checklist (v1.0.0)

Installer file: **T-Books-Setup.exe** (per-user, Start menu, uninstall).

This is a **Windows 10/11 desktop** app (Tauri 2 + Rust + React + SQLite). It is not a Vercel / browser site. There is **no auto-updater** in v1: ship a new `T-Books-Setup.exe` when you release.

## Install on an office PC (no Node, no Rust)

Give people **only** `T-Books-Setup.exe`. They do not install Node, Rust, npm, or Visual Studio. Those exist only on the machine that **builds** the installer.

On a fresh Windows 10/11 PC, Setup:

1. Copies the T Books app (the Tauri `.exe` plus UI assets). Visual C++ runtime DLLs ship next to that binary so nobody hunts for a redistributable.
2. Checks for **Evergreen WebView2**. Windows 11 usually already has it; Windows 10 often does not. If it is missing, Setup **downloads** Microsoft’s WebView2 bootstrapper from Microsoft (`https://go.microsoft.com/fwlink/p/?LinkId=2124703`) and runs it silently (needs internet for that step). If the first pass still finds no WebView2, Setup retries the same download after files are copied. If WebView2 is already there, nothing extra is downloaded.
3. Does **not** install Node, Rust, Chrome, or Office. Does **not** unpack `credentials.json`. Those are not app runtimes.

Books, backups, and logs stay in **`%LOCALAPPDATA%\T-Books`** (hyphenated). That folder is created when the app first runs, not as a secret baked into Setup. Uninstall does not delete it.

Need internet **only** if WebView2 is missing (or later, for Access / Refresh / Submit). The register itself is offline-first after install.

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

Need on the **builder** PC only: Windows 10/11, Node 22, Rust stable (`rustup`). WebView2 is required to run the app locally while developing; the office installer downloads it for users who lack it. NSIS is pulled by the Tauri bundler; you do not install Electron.

```
npm ci
npm run tauri:build
```

That runs the Tauri frontend build, compiles the Rust app with the `desktop` feature, builds a **current-user** NSIS installer, then copies it to:

`dist/installers/T-Books-Setup.exe`

Do **not** run `npx vite` or `cargo tauri` without the npm script. `npm run tauri:build` is the supported command (`tauri build` + rename).

Linux / this sandbox **cannot** emit a Windows NSIS `.exe`. Attempted here:

- `npm run tauri:build` (host Linux) — Rust still links GTK (`gdk-3.0` missing). Even with GTK, NSIS is not produced on Linux.
- `tauri build --target x86_64-pc-windows-gnu --bundles nsis` — fails without MinGW (`x86_64-w64-mingw32-dlltool` missing). There is no Windows linker or NSIS (`makensis`) in this environment.

Use a Windows 10/11 PC or the GitHub Actions job `windows-nsis` (`windows-latest`), then download the `T-Books-Setup` artifact. Do not treat a Linux `dist/client` Vite build as the desktop installer.

## After install (each PC)

1. Run `T-Books-Setup.exe` (no admin needed for a per-user install).
2. Open **T Books** from the Start menu folder **T Books**.
3. Sign in as `thotanitin123@gmail.com` and set the password for **this PC**.
4. If Access / Refresh / Submit are needed, copy `credentials.json` into `%LOCALAPPDATA%\T-Books` **after** install. Never bake that file into git or the installer.

Uninstall (Settings → Apps, or the Start menu uninstall entry) removes the program only. Books stay under `%LOCALAPPDATA%\T-Books`.
