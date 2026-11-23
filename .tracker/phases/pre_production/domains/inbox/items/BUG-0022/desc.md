## Problem Description
**Audit Finding:** `SEC-05` (Low) — `docs/codebase-audit-report.md`

**Locations:** `src/tui/ui.rs:100-114, 238-252, 296-415, 563-829`, `src/tui/canvas.rs:616-622, 651-665, 691-751`, `src/tui/modal.rs:357-368`.

Strings loaded from `.rocket/` or external adapter JSON are rendered without stripping C0/C1 control characters, and `modal::is_printable_char` does not filter `c.is_control()`.

**Recommendation:** Strip/replace `char::is_control` on rendered strings (preserving line breaks only where `.lines()` splits) and reject `c.is_control()` in `is_printable_char`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
