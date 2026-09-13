# Google service account (this PC only)

T Books talks to Google as a **service account**. Nobody signs in with a Google user.

## Where the JSON lives

| Place | Used when |
|---|---|
| Environment `TBOOKS_GOOGLE_SA_JSON` | Whole JSON as one string (CI / owner machine) |
| Environment `TBOOKS_GOOGLE_SA_PATH` | Path to a JSON file outside the repo |
| `%LOCALAPPDATA%\T-Books\credentials.json` | Each Windows install (preferred) |
| `secrets/google-service-account.json` | Local workspace copy, **gitignored** |

Copy `google-service-account.example.json` to `google-service-account.json` and paste the real PEM **only on a machine you control**. That file is never committed.

Share every hive spreadsheet and the `Loopbooks hive` Drive folder with the service account email as **Editor**. Enable Google Sheets API and Google Drive API on that Cloud project.

The app never logs `private_key`, never puts it in a sheet, and never sends it to the UI.
