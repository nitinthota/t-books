# T Books — quick start

## A. Office PC (run the app only)

1. Get **T-Books-Setup.exe** from a GitHub Release, the `windows-nsis` Actions artifact, or whoever builds Setup. Run **only** `T-Books-Setup.exe`. Do **not** install Node, Rust, or WebView2 yourself.
2. Double-click Setup (per-user; no admin). The wizard can have several steps. If **WebView2** is missing, Setup installs Microsoft’s Evergreen WebView2 runtime as one of those steps — it downloads Microsoft’s Evergreen WebView2 helper internally (internet may be needed). Visual C++ files are already inside Setup. You never unzip extra files or run a second installer.
3. Open **T Books** from the Start menu folder **T Books**. Books live in `%LOCALAPPDATA%\T-Books` (created on first launch). Uninstall does not delete that folder.
4. Sign in as `thotanitin123@gmail.com` and **set the password for this PC**. Other people sign in with **their office email and their own password on that PC**. There is no Google user login and no personal Google `credentials.json`.
5. **Bot file (hive only):** after install, copy the Google **service account** JSON to `%LOCALAPPDATA%\T-Books\credentials.json` on each PC that must Access / Refresh / Submit. Same bot file on every such PC. The person who copies it still signs in with email + password. Never put that JSON in Setup, git, or Program Files.
6. **Refresh** when online to pull vouchers. Sales, HR, inventory, logistics stay local and are never deleted by Refresh.
7. Open a voucher. Value, paid, still to pay are on that screen. Payments stay on the same voucher (max 5).
8. **Submit to Google** only if you are owner/admin. A mismatch with the sheet is a conflict — reload or cancel. No silent overwrite. Do **not** type live bills into `Voucher_Raw_Data` (read-only archive).
9. **Settings → Backup Data** before a risky restore or a new PC.

### Bot vs human credentials

| Who | What they use | Where it lives |
|---|---|---|
| **Bot** | One Google Cloud **service account** JSON (typical name `credentials.json`). This is the hive bot, not a person. | `%LOCALAPPDATA%\T-Books\credentials.json` on that Windows user profile. Needed for Access / Refresh / Submit. |
| **Human** | Office email + password **on that PC** (hashed locally). Access sheet: Name, Email, Role, Active (and Account_Type). Credentials tab is **Password_Set** Yes/No only — **no passwords in Google**. | Password is **not** in the sheet and **not** a second Google JSON. Humans do **not** place a personal Google OAuth client file. |

Internet is needed when WebView2 is missing at install, and later for Access / Refresh / Submit. Looking at the books after that is this PC only.

## B. Builder PC (only if you produce Setup.exe)

Need Windows 10/11, Node 22, and Rust. Office people skip this.

```
npm ci
npm run tauri:build
```

That writes `dist/installers/T-Books-Setup.exe`. Details: docs/RELEASE.md. Do not bake the service-account JSON into Setup.
