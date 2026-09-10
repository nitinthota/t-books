# Sync design

Google is the office register. Each PC is independent until Submit. There is **no auto sync** and **no background poll**.

## Refresh (sheet → this PC)

Button on the board. Reads `Voucher_Raw_Data` once.

1. If this PC is offline → fail with a real error. Do not fake an empty list.
2. If any local voucher is **dirty** → **stop**. Show voucher numbers.  
   **Keep local** or **Discard local & Refresh**. Never silent overwrite.
3. Upsert by voucher number (column A = integer). Do not wipe the table.
4. Skip bad rows; log voucher number + error type only (no amounts, GST, bank).
5. Re-run calcPo / totals / status after import.
6. **Never** delete or rewrite sales, purchase, HR, inventory, logistics, documents.

Access Refresh is a separate command: reads the Access tab into `access_cache`. Same fail-safe: keep old cache on error.

## Submit (this PC → sheet)

One voucher per click. Owner/admin only.

1. Offline → fail, keep dirty.
2. Fetch **that row only** (find column A, then that A1 range).
3. Compare SHA-256 **fingerprint** of the Google row to the last known `source_hash`.
4. Match (or empty remote) → write that row. Then clear dirty and store the new hash.
5. Mismatch → **do not write**. Message:

   ```
   Voucher {number} was updated by another user.
   Reload and submit again.
   ```

   Options: Reload from sheet / Cancel.

Different voucher numbers never block each other. A number already used on the sheet for a *new* local voucher is rejected.

## Dirty flag

Local edit → `is_dirty = 1`. Submit success → `0`. Refresh skips dirty rows unless the user discards them.

## Fingerprint

Canonical string of voucher fields + up to 5 payment blocks, then SHA-256. Implementation: `src-tauri/src/core/business_rules/fingerprint.rs` (and the TypeScript twin). Change the hash only by editing that module.
