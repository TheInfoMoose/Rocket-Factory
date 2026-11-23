## Problem Description
**Audit Finding:** `BUG-03` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:1158-1209`.

`UpdateComponent` persists `candidate`, optional `sub_comp`, and optional `cfg`. If `sub_comp` or `cfg` fails after `candidate` succeeds, the `Err` branch leaves `self.project` unchanged in memory without rolling back `candidate` (or deleting `sub_comp`) on disk.

**Fix:** Restore previous component and delete `sub_comp` on partial failure.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
