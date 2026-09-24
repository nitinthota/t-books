# Post order

1. Vendor name + GST + primary bank → Vendors (key = vendor name).
2. Each extra bank → Vendor banks (key = vendor#1, vendor#2).
3. Each PUR line → Purchase lines (key = PUR-n#1).
   Each SAL line → Sales lines (key = PO@job#1).

Parent Post calls related_keys after a good CAS.
Vendor card: Save draft stays here. Post party writes Vendors then extra banks.
Purchase Post bill writes PUR-n then Purchase lines.
Sales Post order writes PO@job then Sales lines.
