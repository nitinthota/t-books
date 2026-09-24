# 2026-09-24 — Sales buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-24-purchase-live-gate.md](2026-09-24-purchase-live-gate.md)
Does not change the sales card.

## How

Save sales order parks on this PC.
Key is `po@job` — same as hive Submit.
Submit already goes through AMOT via submit_office.

## Criteria

1. Save SAL-0001 on job Alpha → `save / sales / SAL-0001@Alpha / park` then SQLite.
2. Submit offline → refuse. Local order stays.
3. Looking at the card never calls Google.

## Pipeline

```
Save order → record(Save) → Park → SQLite
Submit → record(Submit) → Post or Refuse → old hive write if Post
```
