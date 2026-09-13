# T Books production-release test plan

Windows desktop ERP. Views never call Google. Save = SQLite. Submit / Refresh / Access / provision / migrate = explicit Google. `Voucher_Raw_Data` is **read-only**. Dummy tokens only: `CUST_01`, `VEND_02`, `VOUCHER_1001`, `PUR-0001`, `PAY-0001`, `SAL-0001`. Service account only — no Google user login. Owner: `thotanitin123@gmail.com`. Roles: owner / admin / operator. Viewer fail-closed. Passwords never in sheets.

Automated coverage lives in `src-tauri/tests/` plus `src-tauri` `#[cfg(test)]` modules and `tests/unit/*.test.ts`. Live Google **write** is never claimed unless a service account actually wrote (this plan does not write the raw archive).

Legend: **L** = local SQLite only. **G** = Google (Access, hive tabs, Refresh). **RO** = read-only Google (`Voucher_Raw_Data`).

## Commands (run from repo root)

```text
rustup run 1.88.0 cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
rustup run 1.88.0 cargo test --manifest-path src-tauri/Cargo.toml --locked --tests
node --experimental-strip-types --test tests/unit/hive-plan.test.ts tests/unit/modules.test.ts tests/unit/rbac_and_banner.test.ts tests/unit/release.test.ts src/lib/t-books/business_rules/business_rules.test.ts src/lib/t-books/vouchers.test.ts
```

Skip the endurance/load **bins** on a short agent run. Bounded cargo stress (`two_thousand_vouchers_*`, mixed thousand-row parse) is in `--tests`.

Optional live READ/WRITE: set `TBOOKS_GOOGLE_SA_JSON` or drop a service-account file where T Books looks. Set `TBOOKS_LIVE_GOOGLE=1` to **require** live Google tests to pass. Never write `Voucher_Raw_Data`. Dummy keys only.

---

## Per module / screen

| Module | Screen actions | Google? | Expected | Automated |
|---|---|---|---|---|
| Sign in | Inspect email, set first password, login, logout | G only if online Access prefetch | Owner always allowed. Unknown / inactive / viewer denied. Short password rejected. Wrong password rejected. | `functional`, `auth.rs`, release_matrix |
| Board | Counts, last error, Refresh, Force refresh, dirty list | Refresh = G | Dirty vouchers stop Refresh and list numbers. Force refresh replaces register only. Failure keeps local. Banner exact when offline. | `vouchers.rs`, `recovery`, release_matrix |
| Vouchers | Filter list, open, Save, add payment, Submit, Reload | Save = L. Submit/Reload = G | Integer column A. Blank vendor rejected. Dummy `VOUCHER_*` rejected. Max 5 payments. Dirty after save. Submit CAS / conflict. | voucher_edit, writeback, invalid_data |
| Finance | Trial, FY picker | L | Trial from local PO/payroll; balanced when books balance. | `trial.rs` |
| Purchase | Save simple/contract PO, payment, delete, Submit | Save = L. Submit = G | Unique bill number. Blank vendor rejected. Alloc `PUR-0001` / `PAY-0001`. Missing hive tab keeps dirty local. | office.rs, writeback, office_sync |
| Sales | Save PO + items, preview calcPo, Submit | Save = L. Submit = G | Unique PO per project. Negative qty allowed. Hive key `PO-1@CUST_01`. | office.rs, office_sync |
| Vendors | List, search, open (vouchers + projects) | L | From local cache / office rows. Refresh does not wipe office. | office search, release_matrix |
| Projects | List, search, detail | L | Same. | release_matrix |
| Inventory | Save, move, delete, Submit | Save = L. Submit = G | Required item name. Odd qty/cost coerce finite. Missing tab keeps local. | release_matrix |
| Logistics | Save, move, delete, Submit | Save = L. Submit = G | Vehicle required. | release_matrix |
| HR | Person, payroll Save, Submit salary | Save = L. Submit = G | Active person needs salary. One salary / person / month. `SAL-0001`. | office.rs, hr_payroll.rs |
| Documents | Save path, open, Submit | Save = L. Submit = G | Name or path required. Open missing file is a real error (no wipe). | release_matrix |
| Duplicates | List, merge | L | Lookalike vendors merge selected name. | control.rs |
| Explorer | Open table | L | Unknown table error. No password/token columns. | control.rs |
| Rules | List, toggle | L | Seeded rules; toggle persist. | control.rs |
| Access | Refresh, write row (owner) | G | Empty sheet is a real error. Viewer cannot open. Owner write needs creds/online. Passwords never in cells. | access.rs, hive_plan credentials headers |
| System / hive | Backup, restore, provision, migrate dry-run, bootstrap tab, hive status | Backup = L. Provision/migrate/bootstrap = G | Bad zip does not wipe. Provision without creds reports error, does not panic. Dry-run migrate needs Google to **read** raw only. | ops.rs, provision, migrate (memory hive) |

## Per button / command

| Button / command | Google? | Pass | Fail |
|---|---|---|---|
| Inspect / Login | Access prefetch if online | Session role matches Access | Denied message is real |
| Set password | Same prefetch | Hash stored locally, never sheet | Length &lt; 8, mismatch, email-as-password |
| Refresh Access | G | Cache replaced | Offline / no creds → keep cache, show error |
| Refresh vouchers | G (raw READ) | Import register; skip dummy/dotted/blank vendor | Dirty guard lists numbers; no silent overwrite |
| Force refresh | G (raw READ) | Discards dirty, replaces register | Network error keeps local |
| Save voucher / payment | L | Dirty flag, totals | Blank vendor, dotted 20.1, dummy serial, 6th payment |
| Submit voucher | G (hive voucher tab, **not** raw) | Fingerprint match | Conflict copy; network keeps dirty |
| Submit office kinds | G hive | CAS Ok first PC | Second PC conflict, no LWW |
| Access write | G Access tab | Owner only, one row | Viewer/operator denied; no password field |
| Bootstrap hive tab | G | Headers only | Non-owner denied |
| Provision hive | G | Create missing writable tabs | Offline/no creds → report.error set |
| Migrate dry-run | G **READ** raw | Plan counts; dummy skipped | No creds → error; never writes raw |
| Backup / Restore | L | Round-trip vouchers | Corrupt zip keeps books |
| Search | L | Hits vendor/project/inventory/logistics | Empty query ok |
| PO save uniqueness | L | Second same number errors | — |
| HR salary constraints | L | Active + salary &gt; 0; unique month | — |

## Invalid data

| Case | Expected |
|---|---|
| Blank vendor on sheet | Skip row, keep valid rows |
| Dotted `20.1` | Not a voucher |
| `VOUCHER_1001` dummy | Skip / not books |
| Non-integer A (`ABC`, `-9`) | Skip |
| Duplicate PO same project | Error, first kept |
| Short password | Error |
| Viewer mutate | `require_write` error |
| Empty Access sheet | Parse error, owner can still enter |
| Odd money cells | Coerce to 0 on import (`amount_or_zero`) |

## Stress

| Case | Bound |
|---|---|
| Mixed sheet ~1000 rows parse + apply | No panic; skipped &gt; 0; valid kept |
| 2000 vouchers × 5 payments | Apply &lt; 30s; RSS &lt; 1.5 GB |
| `t-books-stress` bin | Optional; cap `TBOOKS_STRESS_VOUCHERS` |

## Google Sheets (MemoryHive / MemorySheet)

Already in `hive.rs`, `submit.rs`, `writeback.rs`, `office_sync.rs`, `tests/live_sheets.rs`: CAS match, conflict, missing tab keeps local, failed write leaves outbox. `sheets.rs` refuses raw writes without calling Google. Optional live **READ** of raw. Optional live **WRITE** of dummy keys on writable hive tabs (`Purchase`, `Purchase payments`, `Sales_PO`) when a service account is present; skipped otherwise. Drive MCP can list/read hive workbooks but cannot PUT cell values.

## Pass / fail log

Recorded on this branch after the suites below. Dummy tokens only. **No live Google writes succeeded** (no service-account JWT in this environment). Google Drive MCP can **read** hive workbooks as the connected Drive user but has **no cell-write API**. Grok app-data Sheets tools were not in the tool list.

| Suite | Result |
|---|---|
| `rustup run 1.88.0 cargo test --locked --lib` | **pass** 126 / 0 fail |
| Integration (functional, data_corruption, stress, multi_user, recovery, writeback, invalid_data, release_matrix) | **pass** 51 / 0 fail |
| `tests/live_sheets.rs` | **7 passed** — logic + multi-user Memory hive; live write **skipped** (no `TBOOKS_GOOGLE_SA_JSON` / credentials.json) |
| Endurance + load cargo tests | skipped (time cap) |
| `t-books-stress` / `t-books-endurance` bins | skipped (time cap); bounded 2k-voucher stress test passed |
| Node: hive-plan, modules, rbac, release, business_rules, vouchers | **pass** 36 / 0 fail |
| Live Google READ raw | **skipped** — no service account in this environment |
| Live Google write (hive tabs) | **skipped** — no service account; Drive MCP cannot PUT cells. Not claimed as success. |

Optional: set `TBOOKS_LIVE_GOOGLE=1` plus a service account to require the raw READ. Never write `Voucher_Raw_Data`.

Do not paste GST, bank, amounts, or real names into this file from a live sheet.
