# 2026-09-23 — Step 5: Remaining books

Status: **IN PROGRESS**
Adds to: [2026-09-23-step-4-sales-module.md](2026-09-23-step-4-sales-module.md)

## How

Every book uses the same pattern as Purchase and Sales.
A count or a name opens only that slice. Looking never posts.

## Criteria

1. Board tile opens that register.
2. Not posted row opens that book and that key.
3. Job count opens vouchers / purchase / sales for that job only.
4. Vendor count opens vouchers / purchase / sales for that party only.
5. Voucher card vendor opens vendor card. Project opens job card.
6. Finance year picker adds that year only.
7. Inventory and logistics still belong to a job name.
8. Documents still point at voucher / job / order.
9. Views never call Google.

## Pipeline

```
Board tile → register
Not posted PUR-0004 → Purchase filtered to PUR-0004
Job card count → that book, that job
Vendor card count → that book, that vendor
Voucher card name → vendor or job card
```

## Next

Wire inventory and logistics project names the same way as the purchase vendor name.
