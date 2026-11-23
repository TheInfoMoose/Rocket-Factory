## Problem Description
**Audit Finding:** `BUG-11` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/storage.rs:239-283`.

`load_project` reads all `*.toml` in `.rocket/components/` and loads `{component.id}.md` without verifying `file_stem == component.id` or uniqueness of `component.id`. Mismatched or duplicate files create duplicate entries in `Project.components` and break `save_component`/`delete_component`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
