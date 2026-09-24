# 2026-09-24 — Voucher list carries JEF

Status: **IN PROGRESS**
Adds to: [2026-09-24-jef-picture.md](2026-09-24-jef-picture.md)
Card layout unchanged. Extra fields on the same JSON row.

## How

`list_vouchers` / `get_voucher` / `save_voucher` still load SQLite.
Then JEF pins the last event for that number onto the same row:
`deskIntent`, `deskDecision`, `deskAt`.

Save 20 → list row 20 has `deskDecision = park`.
No Google.

## Criteria

1. Same columns as before (number, vendor, job, money, status).
2. After Save, desk fields match the zef event for that number.
3. A clean row with no event has empty desk fields.

## Next

One quiet line on the voucher card when deskDecision is park or refuse.
Not a new screen.
