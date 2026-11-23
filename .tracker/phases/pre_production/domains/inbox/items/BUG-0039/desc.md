## Problem Description
**Audit Finding:** `BUG-16` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/model.rs:64-120`.

`Component`, `ProjectConfig`, `FeatureItem.pushed`, and `Connection.kind` omit `#[serde(default)]` on most fields, breaking minimal hand-authored TOMLs. `GridConfig` accepts `cols = 0` or `rows = 0`, blocking card creation when `last_canvas_capacity` is unset or smaller.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
