# MODULE_GAP_VPS — Vouchers, Purchase, Sales

After `parity/vps-1to1`. Dummy ids only: CUST_01, VEND_02, VOUCHER_1001, PUR-0001, PAY-0001.

Views still SQLite only. Save = local dirty. Submit = one hive row CAS.

## V Vouchers

Desktop list filters are the six Loopbook strings, multi-select.
Open editor writes header + up to 5 payment blocks via `save_voucher` (SQLite, dirty=1).
Sixth payment rejected with “This voucher already has 5 payments.”
Submit stays `submit_voucher` CAS. Back is top-left.

## P Purchase

List filters Advance / Partial / Fully Paid / Missing Tax Invoice.
Columns include PUR, vendor, project, type, tax inv, goods received, grand, paid, balance, status, dirty.
Editor sections Basic / Contract / Items / Summary.
Type labels: Techsol Purchase (`po`) and Project Expenses (`non_po`).
PAY editor sends Advance / Against Ref / On Account. Over-pay uses `paymentOverTotal`.
Delete reason must be >= 3 characters.

Merge / unmerge pane still later (rules live in purchase-status.ts).

## S Sales

One PoEditor with Basic / Contract / Items / Summary.
List shows PO, project, client, GST, subtotal, GST amt, grand, received, balance, dirty.
`calcPo` preview equals persist. Received may be negative.
Submit key is `PO@project`.
