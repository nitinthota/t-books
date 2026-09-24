# 2026-09-24 — Document buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-24-hr-live-gate.md](2026-09-24-hr-live-gate.md)
Does not change the paper card.

## How

Save paper parks as `document / DOC-{id}` — same key as hive Submit.
New paper with no id yet uses `DOC-new-{name}`.
Submit already goes through AMOT via submit_office.

## Criteria

1. Save DOC-1 → `save / document / DOC-1 / park` then SQLite.
2. Submit offline → refuse. Local paper stays.

## All Save buttons now park

Voucher, purchase, payment, sales, inventory, logistics, people, salary, document.

Next: JEF — the list you see is rebuilt from zef_events + last Refresh. Screens still never call Google.
