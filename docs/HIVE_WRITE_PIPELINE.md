# Hive write pipeline — vendor, banks, lines

Older pipeline notes stay. This page adds the child-row walk.

## Law

Look = this computer.
Save = this computer + dirty.
Post / Submit = one hive row, fingerprint + rev.
Mismatch = do not write. Message: `{KEY} was just updated by another user. Reload and submit again.`

## Order on Post

1. Vendor
   - Row on `Vendors`: name, GST, primary bank, account, IFSC.
   - Then one row per extra bank on `Vendor banks`. Key = `Vendor#1`, `Vendor#2`.
   - Primary bank is never copied onto Vendor banks.

2. Purchase bill
   - Row on `Purchase orders` for `PUR-n`.
   - Then one row per line on `Purchase lines`. Key = `PUR-n#1`.

3. Sales order
   - Row on `Sales_PO` for `PO@job`.
   - Then one row per line on `Sales lines`. Key = `PO@job#1`.

## Who decides

`office_children.related_keys` lists children.
`office_sync.submit_office_with` posts the parent, then each child.
A child never lists further children.

## What the sheet keeps

Vendors workbook: tab Vendors + tab Vendor banks.
Purchase lines workbook: tab Purchase lines.
Sales lines workbook: tab Sales lines.

Archive `Voucher_Raw_Data` stays read-only.

## Buttons

Vendors: Save on this PC / Post party
Purchase card: Save lines / Post bill
Sales card: Save lines / Post order
