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
