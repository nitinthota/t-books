# Pipeline docs (append-only)

This folder is the source of truth for the read/write mind.

## Rules

1. **Never delete or rewrite an older file.** Add a new dated file.
2. Each change = one new file: `YYYY-MM-DD-short-name.md`.
3. Every file has the same four sections: How / Criteria / Pipeline / Status.
4. Status is one of: `PROPOSED` → `IN PROGRESS` → `SHIPPED` → `SUPERSEDED`.
5. When a pipeline changes, the new file states what it replaces and links to the old one.
6. Any AI or human building this must read files newest-first, then the index.

## Index

| Date | File | What | Status |
|---|---|---|---|
| 2026-09-24 | [2026-09-24-event-log-mind.md](2026-09-24-event-log-mind.md) | ZEF / AMOT / JEF event-log mind | PROPOSED |
| 2026-09-23 | [2026-09-23-step-1-amot-decide.md](2026-09-23-step-1-amot-decide.md) | Step 1: decide() + zef_events table | IN PROGRESS |
| 2026-09-23 | [2026-09-23-step-2-voucher-gate.md](2026-09-23-step-2-voucher-gate.md) | Step 2: voucher Save/Submit/Refresh through decide() | IN PROGRESS |
