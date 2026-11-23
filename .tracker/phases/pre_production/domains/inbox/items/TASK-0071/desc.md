## Problem Description
**Audit Finding:** `ARCH-03` (Medium) — `docs/codebase-audit-report.md`

**Locations:** `src/tui/modal.rs:251-256, 491`, `src/tui/app.rs:1111`, `src/archetype.rs:65-74`.

`ModalState::edit_component` normalizes `comp.kind` via `archetype_for(comp.kind).kind`, coercing legacy variants (`UiView`, `DataStream`, `ServiceCore`, `CliTool`, `Storage`) to broad archetypes (`Interface`, `Logic`, `Data`) on submit and silently changing future `export::map_component_type` output.

**Recommendation:** Preserve original `ComponentKind` unless the user explicitly changes the archetype field.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
