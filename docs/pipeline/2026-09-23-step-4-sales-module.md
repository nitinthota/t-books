# 2026-09-23 — Step 4: Sales module

Status: **IN PROGRESS**
Adds to: [2026-09-23-step-3-purchase-module.md](2026-09-23-step-3-purchase-module.md)
Does not replace `sales-screen.tsx`.

## How

Sales is an order book. The list and the order card read SQLite only.
Click the job name to open that job. Purchase bills on the same job are listed on the card.

## Criteria

1. Open SAL-0001 → lines on that order only.
2. Project name opens the job card.
3. Job card Sales count opens sales filtered to that job.
4. Related purchase bills on the same job are visible, not editable here.
5. Save lines parks the order on this PC.
6. Full edit stays on `sales-screen.tsx`.
7. Views never call Google.

## Pipeline

```
Job card → Sales count → Sales list (that job)
Sales list → order row → Sales card (lines)
Sales card → Project → Job card
Save lines → decide(Save) → Park
Submit → Full edit
```

## Next

Step 5: Voucher card names open vendor and job.
