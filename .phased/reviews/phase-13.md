# Phase 13 review

Range: `054f26105cb52c1859b2fe3b109329dbd30572fd..52dcc88f847781391fd4e1730d80d59e2489425e`

## Independent pass

Zero findings.

Traced:
- `src/tui/app.rs`:
  - `move_selected_card` (`src/tui/app.rs:504-608`): `Direction::Right` and `Direction::Down` clamp with `max_col.max(current_pos.col)` / `max_row.max(current_pos.row)` so cards already beyond `effective_grid()` do not jump backwards; `save_project_config` failure rolls back the component via `save_component(&self.root, &previous)` before mutating `self.project.components` or `self.project.config`.
  - `NewComponent` modal submit (`src/tui/app.rs:973-1066`): `save_project_config` failure rolls back the newly written component via `delete_component(&self.root, &comp.id)` and keeps the modal open with the error message before mutating `self.project.components` or `self.project.config`.
  - Unit tests (`src/tui/app.rs:1957-1984`, `3300-3517`): `render_app(&mut app, 140, 40)` replaces hardcoded `last_canvas_capacity` in existing creation tests, and rollback / out-of-grid move tests exercise real filesystem permission failures and key events.
- `src/tui/canvas.rs`:
  - `card_rect` (`src/tui/canvas.rs:110-172`): allows `rel_col > 0` or `rel_row > 0` cards to render at full `card_width` / `card_height` when `rel_col == 0` is clamped on narrow/short canvases (`inner.width >= 4 && inner.height >= 3`), rather than restricting clamping to `(0, 0)` only.
  - `clamp_viewport_scroll` (`src/tui/canvas.rs:174-185`): clamps `scroll_col` and `scroll_row` to `occupied_cols.saturating_sub(vis_cols)` and `occupied_rows.saturating_sub(vis_rows)`.
  - Unit tests (`src/tui/canvas.rs:1171-1366`): narrow canvas row-1 rendering, zoom-out scroll pullback, and Compact L-bend edge routing.
- `src/tui/ui.rs`:
  - `render` (`src/tui/ui.rs:54-76`): computes `occupied_cols`/`occupied_rows` and calls `clamp_viewport_scroll` before `ensure_visible`.
  - `format_component_list_label` (`src/tui/ui.rs:147-188`): truncates `name` with `…` when needed so ` ({complete}/{total})` is preserved within `inner_width`.
  - `wrapped_line_count` and `render_modal` (`src/tui/ui.rs:492-531`, `659-860`): uses compact Help columns when `area.width < 80` (total width 57 cols, fits 68–79 cols without wrapping) and accounts for wrapped line count in `popup_height` clamped to `area.height`.
  - Unit tests (`src/tui/ui.rs:1080-1505`): real `?` / `+` key events and narrow help / component list rendering.
- `tests/e2e.rs`:
  - `select_component_by_id` (`tests/e2e.rs:726-756`): drives `KeyCode::Down`/`Up` instead of mutating `selected_index` directly.
  - `e2e_canvas_zoom_levels_and_compact_4x4_capacity` and `e2e_top_center_default_placement_and_grid_expansion_persists` (`tests/e2e.rs:1113-1228`): verify zoom-out scroll pullback and 1x1 grid expansion persistence.
- `cargo test --workspace`: 116 unit tests + 10 e2e tests passed.

Not checked:
- Interactive terminal rendering in a live TTY (only `TestBackend` buffer assertions were run).
- `cargo clippy` (not run during independent pass).

<!-- SEALED: Independent pass locked -->

## Contract pass

- **Contracts & Must still work:** All `Contract & Behavior`, `Must still work`, `Drops`, and `Tests` items across `STEP 13A`, `STEP 13B`, `STEP 13C`, and `STEP 13D` are met.
- **Scope & Deviations:** Zero unrequested changes, zero deferrals, zero amendments.
- **Verification:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (116 unit + 10 e2e) all exit `0`.
- **Resolved `neb` items ready to close (after user approval):**
  - `BUG-0011`, `BUG-0012`, `BUG-0013`, `BUG-0014`, `BUG-0015`, `BUG-0016`, `BUG-0017`
  - `TASK-0060` (accepted in plan), `TASK-0061`, `TASK-0062`

## Handoff

- **Verdict:** PASS (0 independent findings, 0 contract deviations).
- **Close on approval:** `BUG-0011`, `BUG-0012`, `BUG-0013`, `BUG-0014`, `BUG-0015`, `BUG-0016`, `BUG-0017`, `TASK-0060`, `TASK-0061`, `TASK-0062`.
- **Open items remaining:** `FEAT-0002`, `FEAT-0003`, `FEAT-0004`, `FEAT-0005` (no deferred steps).
- **Decisions for user:** Approve closing the 10 resolved Phase 12 review items above.

