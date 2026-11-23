## Problem Description
**Audit Finding:** `BUG-23` (Low) — `docs/codebase-audit-report.md`

**Locations:** `src/main.rs:17, 30-34`, `src/cli.rs:117-126`.

`ExitCode::from(output.exit_code as u8)` truncates `i32`; `RF_POLL_INTERVAL_SECS` silently treats invalid/`0` values as disabled with no warning; `export --format manifest` without `--component` returns `NotFound`, while `--component` on `roadmap`/`json` is silently ignored.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
