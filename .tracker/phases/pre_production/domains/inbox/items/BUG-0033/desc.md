## Problem Description
**Audit Finding:** `BUG-10` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:202, 259, 578, 668, 843, 1026, 1157, 1266, 1338, 1363, 1438`.

Every save branch gates disk persistence on `if self.root.join(ROCKET_DIR).exists()`. If `.rocket/` is missing or deleted mid-session, edits, moves, connections, deletes, theme changes, and pulls silently succeed in memory without saving or warning.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
