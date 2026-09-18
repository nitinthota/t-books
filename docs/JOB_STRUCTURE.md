# T Books structure

A clerk and a later builder should read the same page.

## One rule

A **project** is only a job name.
A **vendor** is only a party name.
A **sales order** and a **purchase bill** are documents that point at a job.
A **voucher** is a supplier bill that points at a job and a vendor.

Names do not contain documents. Documents point at names. A card adds them up.

```
 Board (company home)
    |
    +-- Job card <---- Voucher, Purchase, Sales
    +-- Vendor card <---- Voucher, Purchase
    +-- Sales order card ----> Job + line items
    +-- Purchase bill card ----> Job + line items + PAY
    +-- Finance year ----> trial of that year only
    +-- Duplicates ----> similar names, you pick the name to keep
```

## Board

Six tiles. Each tile opens that register.

- Vouchers — count and still to pay
- Purchase bills — count and unpaid
- Sales orders — count and balance
- Jobs — billed / paid / due by job
- Vendors — party card
- Not posted — parked documents waiting to Submit

Looking at Board does not post.

## Open a card

| From | Click | Opens |
|---|---|---|
| Board | a tile | That register |
| Jobs list | job name | Job card |
| Job card | Vouchers / Purchase / Sales count | That job only |
| Job card | vendor name | Vendor card |
| Vendors list | party name | Vendor card |
| Vendor card | Vouchers / Purchase count | That vendor only |
| Vendor card | job name | Job card |
| Sales list | order | Sales card with lines |
| Sales card | job name | Job card |
| Purchase list | bill | Purchase card with lines and PAY |
| Purchase card | job name | Job card |

## Lines on Sales and Purchase

The card loads the full document, not the list row.

- Item, description, qty, rate, GST %, amount
- Add row
- Delete row
- Save lines parks the document here
- Full edit is client / vendor / tax / Submit

A list row has no lines until the card opens.

## Finance year

Year starts 1 April.

A voucher dated 12.04.2025 is 12 Apr 2025. It belongs to 2025-26.
A voucher dated 31.03.2025 belongs to 2024-25.

Pick a year on Finance. Only that year is added. Debit must equal credit.

## Duplicates

Star Engineering and Star Engg. are the same party until you say so.
Plant 2 Hydraulics and Plant-2 Hydraulics are the same job until you say so.

You pick the name to keep. The other spelling is folded into it.
Two bills with the same voucher number are listed only. They are never merged.

## Example

Job **Plant 2 – Hydraulics**

- Vouchers 4 → 5, 12, 40, 166 only
- Purchase 2 → PUR-0004, PUR-0007 only
- Sales 1 → SAL-0001 only

PUR-0004 card shows hose lines and PAY-00010.
SAL-0001 card shows customer lines. Click the job name to return.

Star Engineering vendor card lists the jobs that party billed.

## Park and post

- Save / Save lines — parks here
- Submit — posts one document to the company file
- Refresh — brings the company file. Parked numbers are named first
- Posted PUR-n / PAY-n are not rewritten

## Do not overwrite

These files keep the full edit screens. A card sits in front. It does not replace them.

- purchase-screen.tsx — tax invoice, goods received, Submit
- sales-screen.tsx — client, terms, Submit
- vouchers-screen.tsx — payments, Submit

## Never true

- A job is not a folder of files
- A vendor does not belong to one job
- Opening a card does not call the company file
- Changing the year does not post
- Duplicate voucher numbers are not merged
