# 2026-09-24 — Purchase buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-24-voucher-live-gate.md](2026-09-24-voucher-live-gate.md)
Does not change the purchase card.

## How

Save bill / Save payment parks on this PC and writes a ZEF row.
Submit office row asks AMOT first. Posted `PUR-n-01` is refused.
Offline Submit is refused. Message stays human.

## Criteria

1. Save PUR-0004 → `save / purchase / PUR-0004 / park` then SQLite.
2. Save PAY-00010 → `save / payment / PAY-00010 / park` then SQLite.
3. Submit PUR-0001-01 → refuse, never rewritten.
4. Submit offline → refuse. Local bill stays.
5. PUR-0004 and PAY-00010 do not block each other.

## Pipeline

```
Save bill → record(Save) → Park → SQLite
Save payment → record(Save) → Park → SQLite
Submit → record(Submit) → Post or Refuse → old hive write if Post
```
