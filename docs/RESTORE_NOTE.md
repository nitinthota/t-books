# lib.rs restore

`src-tauri/src/lib.rs` restored from `5e90e839` with a **build-stable** close handler:
- no `window.state()` lifetime issue
- no extra `)` after the handler (regex includes trailing paren)

Hive map: `rawSource.write` is false; live voucher writes use `workbooks.voucher_register`.
