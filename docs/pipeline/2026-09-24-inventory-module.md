# 2026-09-24 — Inventory module

Status: **IN PROGRESS**
Adds to: [2026-09-23-step-5-remaining-books.md](2026-09-23-step-5-remaining-books.md)
Does not replace Save / Move / Submit.

## How

Stock lives on this PC. A job name on a row opens that job.
Arriving from a job filters to that job only.

## Criteria

1. Counts show items, jobs, stock value from this PC.
2. Search covers item, type, job.
3. Project name opens the job card.
4. focusId as a job name filters the list. As an id opens Edit.
5. Save parks here. Submit is one hive row when the tab exists.
6. Views never call Google.

## Pipeline

```
Job card → Inventory (that job)
Inventory row project → Job card
Save → this PC
Submit → one hive row
```
