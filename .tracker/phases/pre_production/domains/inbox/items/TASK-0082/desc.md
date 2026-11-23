## Problem Description
**Audit Finding:** `TEST-05` (Low) — `docs/codebase-audit-report.md`

**Locations:** `tests/e2e.rs:48-70, 398-400, 491-518, 583-598, 983-989, 1019-1021, 1339-1354, 1392-1445`, `src/adapter.rs`, `src/cli.rs`.

`e2e_readme_documents_release_build_and_tui_features` couples `cargo test` to exact `README.md` headings/table syntax; other tests hardcode `Tab`x5/`Right`x4 counts, assume sort adjacency, or duplicate blocks. Mock adapters spawn `sh -c` with single-quoted path interpolation (Unix-only, breaks on `'` in `TMPDIR`).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
