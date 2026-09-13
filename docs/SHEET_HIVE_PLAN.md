# Sheet hive plan

Validated against the Sheet hive plan agent (`bc-2a7edae8-704c-4c15-b8d8-edd6451f48f4`) plus T Books hive kinds in this repo.

## Locked choices

1. **Live hive** is the Loopbooks Drive tree (`Loopbooks hive`). T Books Submit / Access / office rows go to those workbooks. Local SQLite is still what the UI reads.
2. **`Voucher_Raw_Data`** on spreadsheet `1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE` is **archive only**. The app never writes it. Refresh still *reads* it to import history. New bills go to **Voucher register** + **Payments**.
3. **No Google user login.** One service account is Editor on the hive. App login stays email + password on this PC.
4. **Passwords never go in a sheet.** Access stores Name, Email, Role, Active, Account_Type. Credentials tab stores Password_Set (Yes/No) only. Argon2id hashes stay on this PC.
5. Purchase / sales / HR / stock / trips have **headers** in Drive. Raw vouchers do not fill them. Those modules stay local until someone Submits a row.

IDs and folder names live in `src-tauri/hive-map.json` (no secrets).

## Service account JSON

Never commit the real file. Load order:

1. `TBOOKS_GOOGLE_SA_JSON`
2. `TBOOKS_GOOGLE_SA_PATH`
3. `%LOCALAPPDATA%\T-Books\credentials.json`
4. gitignored `secrets/google-service-account.json`

Copy `secrets/google-service-account.example.json`. Share every hive workbook and the root folder with the service account as Editor. Enable Sheets + Drive APIs.

## Folders

`Loopbooks hive` → Books, Work, Documents (+ HR, Inventory, Logistics, Projects, Purchase bills, Sales, Vendors, Vouchers), Control, Archive.

## Migration

Owner command: provision missing tabs (headers only), then migrate.

- Source: `Voucher_Raw_Data` (read-only)
- Integer voucher + vendor → Voucher register (project expense if project is set, else purchase bill)
- Occupied payment slots → Payments lines (`{voucher}#{n}`), not new bills
- Unique vendors / jobs / outstanding / board counts
- Dummy (`VOUCHER_*`, `SAMPLE*`, `CUST_*`, `VEND_*`), empty vendor, `20.1` → skip + log
- Sales / payroll / stock / trips → log `unmapped_module`, headers only
- `Migration_Log` records map / skip / error

Dummy tokens in tests only.
