# 2026-09-24 — HR buttons through AMOT

Status: **IN PROGRESS**
Adds to: [2026-09-24-logistics-live-gate.md](2026-09-24-logistics-live-gate.md)
Does not change the people or payslip card.

## How

Save person parks as `people / HR-{id}`.
Save payslip parks as `salary / {salary_number}` — same key as hive Submit.
Submit of a payslip already goes through AMOT via submit_office.

## Criteria

1. Save person 3 → `save / people / HR-3 / park` then SQLite.
2. Save payslip SAL-0001 → `save / salary / SAL-0001 / park` then SQLite.
3. Submit offline → refuse. Local row stays.

## Pipeline

```
Save person → record(Save) → Park → SQLite
Save payslip → record(Save) → Park → SQLite
Submit payslip → record(Submit) → Post or Refuse → old hive write if Post
```
