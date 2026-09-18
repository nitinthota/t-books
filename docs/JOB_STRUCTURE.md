# T Books job structure

A clerk and a later builder should read the same page.

## One rule

A **project** is only a job name.
A **vendor** is only a party name.
A **sales order** is a customer document that points at a job.

Documents are not stored inside those names. They point at them. The card adds them up.

```
 Project (job) <---- Voucher, Purchase, Sales
 Vendor (party) <---- Voucher, Purchase
 Sales order ----> Project, Client
```

## Open a card, then a count

| Card | Click | Opens |
|---|---|---|
| Job | Vouchers / Purchase / Sales | That job only |
| Job | vendor name | That vendor card |
| Vendor | Vouchers / Purchase | That vendor only |
| Vendor | job name | That job card |
| Sales order | Project | That job card |
| Voucher / Purchase row | Project | That job card |

## Example

**Star Engineering** vendor card

- Vouchers 6
- Purchase bills 2 (PUR-0004, PUR-0007)
- Jobs: Plant 2 – Hydraulics, Site A

Click Vouchers 6 → only Star Engineering bills.
Click Plant 2 – Hydraulics → job card.

**SAL-0001** sales card

- Project Plant 2 – Hydraulics (click → job)
- Client, value, received, balance, line count
- Edit when you need to change lines

## Never true

- Project is not a folder of documents.
- Vendor is not owned by one job.
- Looking at a card does not post. Only Submit posts.
