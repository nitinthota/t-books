# Post order

1. Vendor name + GST + primary bank → Vendors (key = vendor name).
2. Each extra bank → Vendor banks (key = vendor#1, vendor#2).
3. Each PUR line → Purchase lines (key = PUR-n#1).
   Each SAL line → Sales lines (key = PO@job#1).

Parent Post must call related_keys after a good CAS.
Purchase screen already posts PUR-n. Sales screen already posts PO@job.
Vendor card needs Post party → submitOffice("vendor", name).
