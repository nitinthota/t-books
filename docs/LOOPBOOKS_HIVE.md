# Loopbooks hive

One folder. Human names on the first row. The app maps those names. The old raw sheet is archive only.

## Folder

Loopbooks hive

```
Loopbooks hive
  Books        company picture and money
  Work         parties, jobs, orders, people, stock
  Control      who may sign in, rules
  Documents    printed copies
  Archive      old raw sheet — do not add new bills
```

## Books

| File | First row | For a clerk | For the app |
|---|---|---|---|
| Loopbooks — Voucher register | Voucher no, Date, Vendor, Project, Tax invoice, What it is for, Bill ₹, Paid ₹, TDS ₹, Due ₹, Status, Vendor bank, Account, IFSC, GSTIN, Remarks, Paper | One bill, one row | Submit / Refresh of a voucher |
| Loopbooks — Payments | Voucher no, Payment #, Date, Amount ₹, TDS ₹, UTR / ref, What it is for, Remarks | 1st payment, 2nd payment | Occupied payment slots |
| Loopbooks — Outstanding | | Who still has to pay | Board tile |
| Loopbooks — Trial balance | | Year trial | Finance |
| Loopbooks — Board | | Company home | Board figures |
| Loopbooks — Result | | Year result | Report |

## Work

| File | First row | For a clerk | For the app |
|---|---|---|---|
| Loopbooks — Vendors | Vendor, GSTIN, Bank, Account, IFSC, Usual project, Notes | Party master | Vendor card |
| Loopbooks — Jobs | Project, Vouchers, Billed ₹, Paid ₹, Due ₹, Customer PO ₹, Notes | Job card | Job counts |
| Loopbooks — Purchase orders | PO number, Date, Vendor, Project, Status, Subtotal ₹, GST ₹, Total ₹, Paid ₹, Balance ₹, Payment terms, Notes | PUR-n | Purchase Submit |
| Loopbooks — Purchase lines | | Hose, qty, rate | Line table |
| Loopbooks — Sales orders | | SAL-n | Sales Submit |
| Loopbooks — Sales lines | | Customer lines | Line table |
| Loopbooks — HR people | | Staff | HR |
| Loopbooks — Payslips | | Month pay | Payroll |
| Loopbooks — Stock | | Items | Inventory |
| Loopbooks — Trips | | Movement | Logistics |

Missing file the app still looks for by tab name: **Purchase payments** (PAY-n against PUR-n). Today voucher payments live on **Loopbooks — Payments**. PAY-n should be its own book next to Purchase orders.

## Control

| File | First row |
|---|---|
| Loopbooks — Access | Name, Email, Role, Active |
| Loopbooks — Rules | When / then |

## Archive — do not add new bills

Workbook **Voucher_Data**, tab **Voucher_Raw_Data**

https://docs.google.com/spreadsheets/d/1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE

One row = one voucher with five payment blocks across the row. That is why a clerk cannot read it as a report.

Move rule:

- Column A integer → Voucher no on Voucher register
- Date, vendor, project, tax invoice, bank, GST → same names on Voucher register
- Each occupied payment block → one row on Payments (Payment # 1, 2, 3…)
- Dotted children (20.1) stay off column A

Checked on 19 Sep 2026: raw tab about 540 rows. Voucher register about 180 rows. Payments about 270 rows. Not all raw bills are on the register yet.

## One name everywhere

Vendor and job names on register, payments, purchase, sales, and master lists must be the same spelling. Keep this name on Duplicates writes this PC then posts each bill.

## What the app must not do

- Do not write new bills to Voucher_Raw_Data
- Do not look at the company file when you only open a card
- Do not replace a posted PUR-n / PAY-n
