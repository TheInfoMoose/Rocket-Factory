## Problem Description
**Audit Finding:** `TEST-01` (High) — `docs/codebase-audit-report.md`

**Locations:** `src/main.rs:1-50`, `tests/e2e.rs`.

`tests/e2e.rs` calls library functions directly without spawning the compiled `rf` binary (`CARGO_BIN_EXE_rf`), leaving `clap` parsing, `default_adapters_path()`, `RF_POLL_INTERVAL_SECS`, stderr error formatting, and `ExitCode` mapping unverified by `cargo test`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
