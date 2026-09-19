# Loopbooks hive

One folder. Human names on the first row. The app maps those names. The old raw sheet is archive only. Do not add new bills there.

## Folder

```
Loopbooks hive
  Books        company picture and money
  Work         parties, jobs, orders, people, stock
  Control      who may sign in, rules
  Documents    printed copies
  Archive      old raw sheet — closed
```

## Books (report + app)

Frozen first row. Filter on. Money columns are rupees.

| File | First row |
|---|---|
| Loopbooks — Voucher register | Voucher no, Date, Vendor, Project, Tax invoice, What it is for, Bill ₹, Paid ₹, TDS ₹, Due ₹, Status, Vendor bank, Account, IFSC, GSTIN, Remarks, Paper |
| Loopbooks — Payments | Voucher no, Payment #, Date, Amount ₹, TDS ₹, UTR / ref, What it is for, Remarks |
| Loopbooks — Outstanding | Who still has to pay |
| Loopbooks — Trial balance | Year trial |
| Loopbooks — Board | Company home |
| Loopbooks — Result | Year result |

One bill = one row on Voucher register. Each payment on that bill = one row on Payments (1, 2, 3…).

## Work

| File | First row |
|---|---|
| Loopbooks — Vendors | Vendor, GSTIN, Bank, Account, IFSC, Usual project, Notes |
| Loopbooks — Jobs | Project, Vouchors, Billed ₹, Paid ₹, Due ₹, Customer PO ₹, Notes |
| Loopbooks — Purchase orders | PO number, Date, Vendor, Project, Status, money columns, Notes |
| Loopbooks — Purchase lines | Item lines for PUR-n |
| Loopbooks — Sales orders / Sales lines | SAL-n and lines |
| Loopbooks — HR people / Payslips | Staff and month pay |
| Loopbooks — Stock / Trips | Items and movement |

PAY-n against PUR-n belongs on a **Purchase payments** tab inside Purchase orders, not on Loopbooks — Payments.

## Control

Access: Name, Email, Role, Active.

## Move check (19 Sep 2026)

Raw tab `Voucher_Raw_Data` has many blank rows.

Usable bills (integer number + vendor):

- Already on Voucher register: **179**
- Already on Payments: **269**
- Extra raw bills that still needed a row: **0**
- Raw rows skipped: blank **874**, no vendor **4**

So every bill that can live on Loopbooks is already there. Nothing left to copy except empty rows.

## Unlink

`hive-map.json` already marks raw `write: false`. The app must not write that tab.

Refresh on the installed build still *reads* raw for history. Live Submit already writes Voucher register + Payments.

New formatted copies could not be uploaded: this Google Drive is out of space. Free space, then replace the four Books/Work files from this PC if you want the frozen header + filter layout.

## What never happens

- New bills on Voucher_Raw_Data
- Opening a card calling Google
- Rewriting a posted PUR-n / PAY-n
