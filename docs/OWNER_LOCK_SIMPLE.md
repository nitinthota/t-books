# T Books — simplified requirements (owner lock)

Working name: **T Books**

This replaces the hive / CAS / dirty-voucher model for day-to-day use. Loopbook remains rules reference only. Desktop app stays Tauri + SQLite; Google Sheets is a plain register, not a multi-PC fingerprint system.

## 1. Sync model — remove hive complexity

**Remove**

- Hive kinds, fingerprint CAS, Submit conflict flows as the normal UX
- Dirty-voucher blocking of Refresh
- Keep Local / Discard Local & Refresh dialog and buttons

**Replace with simple Sheets I/O**

| Action | Behaviour |
| --- | --- |
| Read | Pull from Google Sheets into local SQLite (Refresh) — always force-style |
| Write | Save on this PC; **Save to Google** pushes the row when online |
| Offline | App still opens; local SQLite works; Google read/write paused with a clear error |
| Views | Still read local SQLite only |

**Remove drafts**

- Header control **Remove drafts** clears local-only dirty rows + pending outbox.
- No Keep/Discard dance.

## 2–3. Sales / Purchase UI

- Sales: one **New PO** (no contract PO).
- Purchase: one **New bill** (no contract/simple layer).

## 4. Project expense

- Paid by — employee only + Techsol purchase fields (next).

## Build progress

1. Sales single New PO — done
2. Purchase single New bill — done
3. Remove drafts — done (header + store + vouchers.clearLocalDrafts)
4. Simple Refresh, no Keep/Discard modal — done
5. Save to Google on Sales/Purchase — wiring on screens
6. Project expense — pending
7. Polish + installer — pending
