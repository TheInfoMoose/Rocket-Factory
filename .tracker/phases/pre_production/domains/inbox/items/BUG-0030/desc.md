## Problem Description
**Audit Finding:** `BUG-07` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/canvas.rs:441, 486, 530-540, 651-665, 697-706`.

`truncate_text`, `compute_label_pos`, `badge_len`, and per-char cell placement use `chars().count()` and 1-column increments instead of terminal display width (`unicode_width` / `Line::width`), causing wide CJK/emoji or combining chars to overflow cards and corrupt adjacent cells.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
