# T Books job structure

This is the company map. A clerk and a later builder should read the same page.

## One rule

A **project** is only a job name.

Vouchers, purchase bills, sales orders, and vendors are not stored inside that name.
Each document **points at** the job. The job card **adds up** those documents.

```
                    Project  (job name)
                         ^
         +---------------+---------------+
         |               |               |
     Voucher          Purchase         Sales
     (bill)           PUR / PAY        customer order
         |
       Vendor  (party). One vendor may work on many jobs.
```

## Walk both ways

| From | Click | Opens |
|---|---|---|
| Projects list | job name | Job card: counts and money for that job only |
| Job card | Vouchers count | Voucher list, that job only |
| Job card | Purchase bills count | Purchase list, that job only |
| Job card | Sales orders count | Sales list, that job only |
| Job card | a vendor name | That vendor |
| Voucher | Project name | Same job card |
| Purchase row | Project name | Same job card |
| Sales row | Project name | Same job card |

Other jobs stay hidden after a count is clicked.

## Example

Job **Plant 2 – Hydraulics**

- Vouchers 4 → only 5, 12, 40, 166
- Purchase bills 2 → only PUR-0004, PUR-0007
- Sales orders 1 → only SAL-0001
- Vendors 2 → Star Engineering, Good Luck Hydraulics

Billed / paid / still to pay on the card are those four vouchers only.

Open voucher 5 → click **Plant 2 – Hydraulics** → back on the same job card.

## What is never true

- The project master is not a folder of documents.
- A vendor does not belong to one job.
- Posted PUR-n / PAY-n numbers are not rewritten when you open a job.
- Looking at a job does not call the company file. Only Submit posts.

## Where this lives in the product

| Screen | Role |
|---|---|
| Projects | Job list and job card |
| Vouchers | Supplier bills. Project field on each bill |
| Purchase | PUR bills and PAY. Project field on each bill |
| Sales | Customer orders. Project field on each order |
| Vendors | Party master. Shown on a job only if a bill names them |

## Park and post (unchanged)

- Save parks the document here.
- Submit posts one document to the company file.
- Refresh brings the company file. Parked documents are named first. They do not block other numbers.
