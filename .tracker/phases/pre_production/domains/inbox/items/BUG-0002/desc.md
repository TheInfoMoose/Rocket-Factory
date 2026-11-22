## Problem Description
In src/tui/terminal.rs:125-142, run_event_loop constructs md_path from comp_id and invokes suspend_for_editor without calling validate_component_id or verifying canonical path containment against .rocket/components/ (unlike App::reload_component_markdown and src/storage.rs). Also launches $EDITOR on a nonexistent path if !md_path.exists() and the component is not found in app.project.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
