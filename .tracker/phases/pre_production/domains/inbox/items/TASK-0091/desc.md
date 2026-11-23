## Problem Description
**Audit Finding:** `DOC-01` (Low) — `docs/codebase-audit-report.md`

**Location:** `README.md:120-261`.

`README.md` omits warning about non-defaulted fields in hand-edited component TOMLs, misstates `last_canvas_capacity` scope, omits several keybind/mode details (`a` requiring Navigate, `E`/`D` first-edge behavior, form `↑`/`↓`/`Shift+Tab`, `ThemeSwitcher` `j`/`k`), omits empty `$HOME` theme path behavior, documents discarded `PushResponse`, and lacks `## License`, `## Testing`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
