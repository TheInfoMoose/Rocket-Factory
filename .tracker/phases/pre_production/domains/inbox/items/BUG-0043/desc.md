## Problem Description
**Audit Finding:** `BUG-20` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/tui/canvas.rs:260-288, 314-399`.

Touching/overlapping card pairs reverse `start`/`end` coordinates inside cards; `route_orthogonal_edge` uses unchecked `+ 1` at `314..392` instead of `saturating_add(1)`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
