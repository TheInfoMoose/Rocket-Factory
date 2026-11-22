## Problem Description
`run_tui` in `src/tui/terminal.rs` resolves the initial theme with its caller-supplied `theme_dirs: &[PathBuf]`, then calls `App::new`, which resets `app.theme_dirs` to `default_theme_dirs(&root)` without copying `theme_dirs`. Any library caller passing custom `theme_dirs` will not see those directories when pressing `t`. Set `app.theme_dirs = theme_dirs.to_vec()` after `App::new(...)`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
