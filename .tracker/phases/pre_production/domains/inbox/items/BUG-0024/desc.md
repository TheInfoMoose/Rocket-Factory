## Problem Description
**Audit Finding:** `BUG-01` (High) — `docs/codebase-audit-report.md`

**Location:** `src/tui/ui.rs:190-260`.

`render_component_list` uses stateless `List` without `ListState` (`frame.render_widget(list, area)`). When visible items exceed `area.height.saturating_sub(2)`, moving selection below the fold highlights off-screen rows without scrolling.

**Fix:** Use `render_stateful_widget` with `ListState`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
