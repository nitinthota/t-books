# 2026-09-23 — Step 3: Purchase module

Status: **IN PROGRESS**
Adds to: [2026-09-23-step-2-voucher-gate.md](2026-09-23-step-2-voucher-gate.md)
Does not replace `purchase-screen.tsx`.

## How

Purchase is one book. The list and the bill card read SQLite only.
Click a count or a name to open only that slice.
Save parks the bill. Submit is a different button on Full edit.

## Criteria

1. Open PUR-0004 → lines + payments on that bill only.
2. Payments labelled 1st payment, 2nd payment.
3. Project name opens that job card.
4. Vendor name opens that vendor card.
5. Job card Purchase count opens purchase filtered to that job.
6. PUR-n and PAY-n are different keys. Both may Submit.
7. Posted `PUR-n-01` is never rewritten.
8. Views never call Google.
9. Full edit stays on `purchase-screen.tsx`.

## Pipeline

```
Job card → Purchase count → Purchase list (that job)
Purchase list → bill row → Purchase card (lines + PAY)
Purchase card → Project → Job card
Purchase card → Vendor → Vendor card
Save lines → decide(Save) → Park
Submit bill / PAY → decide(Submit) → Post or Refuse
```

## Next

Step 4: Sales module the same way.
