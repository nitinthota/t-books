# Testing

Run from the repo root.

```bash
# TypeScript
npm run typecheck
node --experimental-strip-types --test src/lib/t-books/business_rules/business_rules.test.ts src/lib/t-books/vouchers.test.ts src/lib/t-books/office.test.ts

# Rust library (calcPo, dirty guard, submit, backup)
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Full T Books suite (unit + integration + stress bins)
npm run test:books
```

`npm run test:books` writes `reports/test_report.json` (gitignored).

## Suites

| Area | Where |
|---|---|
| calcPo / payments / status | `src-tauri/src/core/business_rules/` and `src/lib/t-books/business_rules/` |
| Dirty guard, parse, 1000-row import | `src-tauri/src/vouchers.rs` tests, `tests/` |
| Fingerprint write-back | `src-tauri/tests/writeback.rs`, `tests/writeback/` |
| Backup / restore / semver | `src-tauri/src/ops.rs` tests, `tests/ops/` |
| No secrets in logs | `tests/security/no_secrets.test.ts` |
| Stress / load / endurance | `src-tauri/tests/` + `t-books-stress` / `t-books-endurance` bins |

Tests use dummy tokens only (`VOUCHER_1001`, `CUST_01`, `VEND_02`). No real office rows.

## CI

`.github/workflows/build.yml` installs Node + Rust, typechecks the UI, and runs `cargo test --lib`.
