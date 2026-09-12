# MODULE_GAP_VPS — Vouchers, Purchase, Sales

After `ad5bd72` plus this P/S hive pass. Dummy ids only: CUST_01, VEND_02, VOUCHER_1001, PUR-0001, PAY-0001.

Views still SQLite only. Save = local dirty. Submit = one hive row CAS.

## P Purchase

Desktop editor now has Basic / Contract / Items / Summary, Techsol vs Project Expenses labels, live calcPo totals, PAY method + date + remarks, over-pay block, delete reason >= 3 chars, list columns for grand/paid/balance.

Merge / document link / bank label still later.

## S Sales

Kind contract|project, received (negatives allowed), payment terms via paymentTermDays, live calcPo Summary, clone contract to project PO, Submit kind sales_po key PO@project, dirty list includes sales_po.

Project-screen money rollup still thin.

## C4.4

KIND_SALES_PO tab Sales_PO. office_sync load/mark on Submit. Additive sales_po columns: kind, received_rupees, payment_term_*, is_dirty, hive_rev, source_hash.
