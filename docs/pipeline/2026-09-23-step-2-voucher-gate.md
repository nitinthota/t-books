# 2026-09-23 — Step 2: Voucher gate

Status: **IN PROGRESS**
Adds to: [2026-09-23-step-1-amot-decide.md](2026-09-23-step-1-amot-decide.md)
Does not change screens.

## How

Voucher Save, Submit, and Refresh ask `decide()` before they write.
The answer is stored in `zef_events`. Buttons and labels stay the same.

## Criteria

1. Save still writes SQLite + dirty=1. Also logs `save/park`.
2. Submit still uses the same conflict sentence: `Voucher {n} was just updated by another user. Reload and submit again.`
3. Submit still does not write on mismatch.
4. Refresh with dirty keys still returns the Dirty list. Also logs `refresh/stop_refresh`.
5. Discard local (force refresh) logs `discard_local` then pulls hive.
6. Views still never call Google.

## Pipeline

```
Save     → decide(Save)     → Park   → SQLite dirty=1
Submit   → lookup hive fp   → decide(Submit) → Post or Refuse
Refresh  → list dirty keys  → decide(Refresh) → Stop or Apply
```

## Next

Step 3: same gate for Purchase Save / Submit / Refresh.
