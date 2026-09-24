# 2026-09-24 — Logistics buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-24-inventory-live-gate.md](2026-09-24-inventory-live-gate.md)
Does not change the trip card.

## How

Save trip and move to job park on this PC.
Key is `TRIP-{id}` — same as hive Submit.
New trip with no id yet uses `TRIP-new-{vehicle}`.
Submit already goes through AMOT via submit_office.

Drive still needs the Logistics tab on Loopbooks — Trips. Save does not write Drive.

## Criteria

1. Save TRIP-1 → `save / logistics / TRIP-1 / park` then SQLite.
2. Move TRIP-1 to a job → same park, then SQLite.
3. Submit offline → refuse. Local trip stays.
4. Submit with no Logistics tab → old message. Local trip stays.

## Pipeline

```
Save trip → record(Save) → Park → SQLite
Move trip → record(Save) → Park → SQLite
Submit → record(Submit) → Post or Refuse → old hive write if Post
```
