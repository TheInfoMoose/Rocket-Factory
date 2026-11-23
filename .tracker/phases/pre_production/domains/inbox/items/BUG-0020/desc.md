## Problem Description
**Audit Finding:** `SEC-03` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/storage.rs:70-105, 203, 245-268`.

`load_project` calls `ensure_contained` on `{component.id}.md`, but reads `.rocket/project.toml` and `.rocket/components/*.toml` directly without calling `ensure_contained` or checking regular-file type. Also `ensure_contained` gates canonicalization on `resolved.exists()`, which returns `false` for dangling symlinks.

**Recommendation:** Enforce `ensure_contained` and regular-file checks for `project.toml` and `*.toml` reads in `load_project`, and check `resolved.symlink_metadata().is_ok()` in `ensure_contained`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
