## Problem Description
**Audit Finding:** `PKG-01` (Medium) — `docs/codebase-audit-report.md`

**Locations:** `Cargo.toml:1-21`, `Cargo.lock:3`.

`Cargo.toml` omits `rust-version` (`cargo metadata` reports `rust_version: None`) even though `Cargo.lock` uses format `version = 4` and resolved crates pulled in by `ratatui 0.29.0` require Rust `1.88.0` (`darling*` `0.24.1`, `instability 0.3.14`).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
