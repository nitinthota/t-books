# 2026-09-24 — Voucher buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-23-step-2-voucher-gate.md](2026-09-23-step-2-voucher-gate.md)
Does not change screens.

## How

Save / Submit / Refresh on vouchers call `record()` first.
The old write still happens. The event is kept on this PC.

## Criteria

1. Save parks. Event `save / voucher / {n} / park`.
2. Submit with no role or offline is refused by the same desk. Message unchanged.
3. Refresh with dirty keys is StopRefresh. Local rows stay.
4. Discard local on force Refresh is logged.
5. Views still never call Google.

## Pipeline

```
Save → record(Save) → Park → SQLite
Submit → record(Submit) → Post or Refuse → old hive write if Post
Refresh → record(Refresh) → Stop or Apply → old pull if Apply
```
