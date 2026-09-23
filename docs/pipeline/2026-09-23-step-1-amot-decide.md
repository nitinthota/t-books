# 2026-09-23 — Step 1: AMOT decide + ZEF log

Status: **IN PROGRESS**
Adds to: [2026-09-24-event-log-mind.md](2026-09-24-event-log-mind.md)
Does not replace the design file.

## How

One function, `decide(event, facts)`, answers Save / Submit / Refresh.
One SQLite table, `zef_events`, stores every decision. Rows are only inserted.
Screens are not changed in this step.

## Criteria

1. Save → Park. Works offline.
2. Submit offline → Refuse. Data stays.
3. Submit when hive fingerprint != last known → Refuse with `{KEY} was just updated by another user. Reload and submit again.`
4. Submit when hive empty or fingerprints match → Post.
5. Posted-forever key → Refuse, never rewrite.
6. Two different keys → both may Post.
7. Refresh offline → Refuse. Do not wipe.
8. Refresh with dirty keys → Stop and list those keys.
9. Event log never updates an old row.

## Pipeline

```
Event (save|submit|refresh|keep|discard)
    +
Facts (online, dirty, posted_forever, can_submit, known_fp, hive_fp, dirty_keys)
    → decide()
    → Decision (park|post|refuse|stop_refresh|apply_refresh|keep_local|discard_local)
    → append zef_events
```

Code: `src-tauri/src/core/pipeline.rs`
Table: `zef_events` in schema 12

## Next

Step 2: route voucher Save / Submit / Refresh through `decide()` without changing the screen.
