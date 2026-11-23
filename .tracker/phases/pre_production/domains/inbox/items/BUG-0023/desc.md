## Problem Description
**Audit Finding:** `SEC-06` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/theme.rs:70-162`.

`load_theme_file` reads `.ron` files without a size cap. `resolve_theme` and `list_available_themes` canonicalize `dir` before checking `starts_with(&canon_dir)`, so if `.rocket/themes` is a symlink outside the workspace, containment passes relative to the external target.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
