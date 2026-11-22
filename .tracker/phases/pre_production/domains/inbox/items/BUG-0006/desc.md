## Problem Description
In `src/tui/app.rs` `apply_modal_action`, `ModalAction::UpdateComponent` and `ModalAction::UpdateConnection` close the modal and discard user edits when `save_component` or `save_project_config` fails, whereas `CreateComponent` and `SelectTheme` keep the modal open with `form.error = Some(msg)`. Preserve modal state and display `form.error` on save failure for `UpdateComponent` and `UpdateConnection`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
