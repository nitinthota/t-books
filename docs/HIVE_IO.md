# Hive input and output

Views never call Google.

## Save

Writes this computer only. Marks dirty.

## Post

One parent row. Fingerprint + rev. Mismatch: do not write.

Then child rows, same rule:

| Parent | Child key | Sheet |
|---|---|---|
| vendor | vendor | Vendors (GST + primary bank) |
| vendor | vendor#1, vendor#2 | Vendor banks (extra accounts only) |
| PUR-n | PUR-n#1, PUR-n#2 | Purchase lines |
| PAY-n | PAY-n | Purchase payments |
| PO@job | PO@job#1 | Sales lines |
| voucher | voucher | Voucher register |
| voucher | voucher#1 | Payments |

## Refresh

Reads the same parent sheets. Does not wipe dirty keys. Child sheets are the posted picture.

## Not posted from a card

Jobs counts, Outstanding, Board, Trial, Result. Those are reports.
