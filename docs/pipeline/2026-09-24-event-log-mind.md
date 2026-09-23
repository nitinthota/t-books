# 2026-09-24 — Event-log mind (ZEF / AMOT / JEF)

Status: **PROPOSED**
Replaces: direct screen→Google writes in `docs/architecture.md`, `docs/sync-design.md` (those stay as history).

## How

No screen writes Google or another module's data. Every action becomes an **event**. One decision module reads events and acts. Screens only read the local projection.

Three parts:

- **ZEF** — event log. Append-only table in local SQLite. One row per action.
- **AMOT** — decision maker. Reads each event, applies rules, calls Google only when allowed.
- **JEF** — local projection. Current state rebuilt from events. Screens read this, never Google.

## Criteria

1. Posted numbers (`PUR-n`, `PAY-n`, voucher no.) are never rewritten.
2. A dirty local row wins over Refresh. No silent overwrite.
3. Two PCs posting the same number → refuse, message, no write.
4. Offline → event stays in ZEF, retried when online.
5. Crash mid-write → event log is the source; projection rebuilds from it.
6. Hard rules live in AMOT code. Judgment calls (e.g. name match) may use a model later; rules do not.

## Pipeline

```
Screen action
    │
    ▼
ZEF  append event  (type, key, payload, ts, actor)
    │
    ▼
AMOT decide
    ├─ park locally     → write JEF projection, mark dirty
    ├─ post to Google   → CAS fingerprint check → write sheet → clear dirty
    ├─ refuse           → return reason to screen, no write
    └─ merge names      → rewrite local rows, mark dirty, post later
    │
    ▼
JEF  projection updated
    │
    ▼
Screen re-reads JEF
```

## First module to migrate

Vouchers (already has fingerprint + dirty). Then Purchase, Sales, one at a time.
No screen changes until its event types are defined and tested.
