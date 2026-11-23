## Problem Description
**Audit Finding:** `BUG-12` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/storage.rs:25-68, 146-163`.

`atomic_write` uses predictable `{file_name}.tmp.<pid>` opened with `File::create` (no `O_EXCL`, risking same-process races or symlink truncation) and omits parent directory `fsync` after `rename`. `save_component` writes `.toml` and `.md` sequentially with no rollback if `.md` fails.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
