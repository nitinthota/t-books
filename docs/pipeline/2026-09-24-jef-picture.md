# 2026-09-24 — JEF local picture

Status: **IN PROGRESS**
Adds to: [2026-09-24-document-live-gate.md](2026-09-24-document-live-gate.md)
Does not change cards. Cards still read the old tables.

## How

JEF reads `zef_events` on this PC only.
One row per book + key = last event.
Timeline keeps every event. Nothing is rewritten.

## Criteria

1. Save voucher 20 twice → picture has one row, last park, actor of the second Save.
2. Timeline for 20 has two rows.
3. Purchase picture does not include voucher 20.
4. No Google call.

## Pipeline

```
Save/Submit/Refresh → AMOT record → zef_events
JEF project_book → last event per key
JEF timeline → every event for one key
Card list → still old tables (next slice can switch)
```
