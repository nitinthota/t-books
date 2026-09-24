# Refresh unlinked from archive

Date: 2026-09-24

## What changed
- Refresh of vouchers reads Loopbooks — Voucher register only.
- Reload from sheet reads the same register.
- Archive Voucher_Raw_Data is not opened by Refresh or Reload.
- Archive stays on Drive, write still refused.

## Register columns the app now reads
Voucher no, Date, Vendor, Project, Tax invoice, What it is for, Bill, Paid, TDS, Due, Status, Vendor bank, Account, IFSC, GSTIN, Remarks, Paper.

## Not in this change
- Tab on that file must still be named exactly `Voucher register` (not Untitled).
- Dirty local rows still block Refresh.
- This installer on the PC does not pick this up until a new Setup is built.
