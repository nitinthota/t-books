#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export PATH="/usr/local/cargo/bin:${PATH:-}"

# src-tauri pulls edition-2024 crates; stable must be >= 1.85.
if command -v rustup >/dev/null 2>&1; then
  rustup update stable >/dev/null 2>&1 || rustup update stable
  export PATH="/usr/local/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:${PATH}"
fi

npm ci

# Browser QA (browser-smoke.mjs, agent-browser, computer use).
npx playwright install chromium

# Prefetch Rust crates for cargo test --lib (desktop builds target Windows).
if command -v cargo >/dev/null 2>&1; then
  cargo fetch --manifest-path src-tauri/Cargo.toml
fi
