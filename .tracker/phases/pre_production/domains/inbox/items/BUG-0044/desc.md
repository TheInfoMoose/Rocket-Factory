## Problem Description
**Audit Finding:** `BUG-21` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/export.rs:15-27, 45-118`.

`ComponentManifestExport` omits `parent_id`/`position` and mixes `Contains` edges into `dependencies`. `export_roadmap_markdown` does not escape `|`, `\r`/`\n`, or backticks, and `export_project_json` omits a trailing newline.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
