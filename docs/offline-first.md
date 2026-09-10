# Offline-first

The app must open with no network.

## Banner (exact)

> Offline — working on this PC. Access list and Refresh paused.

Credentials missing:

> Google credentials not found. Running offline.

## Rules

1. **UI reads SQLite only.** Board, voucher open, vendors, projects, search, sales, HR, inventory, logistics, documents — no Google.
2. **Edits save locally** and set dirty on that voucher.
3. **Network is allowed only for:**
   - login Access check when online
   - owner add/disable on Access
   - manual Refresh
   - Submit of one voucher
   - Settings → Check for updates
4. **If Google fails:** keep local data, show the real error. Never wipe the screen to an empty list.
5. **Offline login:** allowed only if that email was accepted on this PC before (cached Access) — except the hardcoded owner, who can always enter on a PC that already has T Books.

## Data folder

`%LOCALAPPDATA%\T-Books`

- `tbooks.db` — books
- `credentials.json` — optional, this PC only
- `logs/` — rotated, no secrets
- `backups/` — last 5 auto zips + manual backups

Uninstall does not delete this folder.
