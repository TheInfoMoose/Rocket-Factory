## Problem Description
In src/tui/modal.rs:597-606, handle_edit_connection_key matches KeyCode::Char('[') and KeyCode::Char(']') when form.candidates.len() > 1 without checking form.active_field == 0. Typing '[' or ']' in the Label field (active_field == 1) on a multi-edge component switches candidates and discards unsaved edits instead of inserting into form.label.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
