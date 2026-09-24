# 2026-09-24 — Inventory buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-24-sales-live-gate.md](2026-09-24-sales-live-gate.md)
Does not change the stock card.

## How

Save stock and move to job park on this PC.
Key is `INV-{id}` — same as hive Submit.
New line with no id yet uses `INV-new-{item}`.
Submit already goes through AMOT via submit_office.

## Criteria

1. Save INV-1 → `save / inventory / INV-1 / park` then SQLite.
2. Move INV-1 to a job → same park, then SQLite.
3. Submit offline → refuse. Local stock stays.

## Pipeline

```
Save item → record(Save) → Park → SQLite
Move item → record(Save) → Park → SQLite
Submit → record(Submit) → Post or Refuse → old hive write if Post
```
