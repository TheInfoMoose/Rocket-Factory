# Rocket Factory — Repo Rules

Rocket Factory is a minimalist retro-futuristic Rust TUI (`ratatui`) development planning platform using `.rocket/` TOML + Markdown blueprints and tool-agnostic external adapters.

## Plan & Workflow
- Plan file: `/workspace/STEPPED_IMPLEMENTATION_PLAN.md` (`**Workflow:** phased`).
- Forbidden paths (never edit): `Rocket Factory.md`, `STEPPED_IMPLEMENTATION_PLAN.md`, `docs/*`, `.phased/*`, `.orrery/*`, `scripts/*`.

## Mandatory Gates
- Base Rust gate: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- Tool-agnostic guard: `src/` and `Cargo.toml` must never reference any specific external tracker by name (`! grep -rqi 'nebulous' src/`).
- No `.unwrap()` or `.expect()` in production non-test code; use `RfError` / `RfResult`.
- Test fixtures must use reserved placeholder data (`192.0.2.0/24`, `example.com`, `testuser`).

## Guardrails (Zero-Tolerance)
- **Path containment parity:** Every code path that reads, writes, deletes, or passes a workspace-relative path to an external process (e.g., `$EDITOR`, adapter commands) must validate IDs (`validate_component_id`) and verify canonical path containment (`canonicalize` + `starts_with`) against the expected workspace root/subdir before performing I/O or launching the process — matching `src/storage.rs`. Never pass unvalidated or nonexistent component paths to external commands.
- **No unwired scaffolding:** Newly introduced or referenced state fields, structs, and helper functions must be wired and actively exercised in production code paths, not left as dead/unmutated state (e.g., a viewport field never updated or a helper only called in tests). If full wiring requires editing files outside `ALLOWED_FILES`, do not silently leave it unwired — file a `pending` Nebulous item and call it out in the report.
- **Out-of-scope findings:** Never fix issues outside `ALLOWED_FILES`. File any bug, security gap, or missing wiring discovered outside scope as a new `pending` Nebulous item (`neb bug|task` + `neb status <ID> pending`) for end-of-phase review.
