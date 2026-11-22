## Problem Description
In `src/tui/ui.rs` `render_modal`, at 80-column terminal width `popup_width` clamps to 54 (inner width 52), causing the 64-character help line `"    n / e / E         New component / Edit component / Edit edge"` to clip on the right because `Paragraph` does not wrap. Shorten help lines or compute popup width from the longest rendered line capped by `area.width.saturating_sub(2)`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
