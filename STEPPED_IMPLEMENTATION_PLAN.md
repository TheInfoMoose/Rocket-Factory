# Rocket Factory — Stepped Implementation Plan

## Step Execution Roadmap

**Workflow:** phased

**Base gate:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

**Forbidden paths:** `Rocket Factory.md:STEPPED_IMPLEMENTATION_PLAN.md:docs/*:.phased/*:.orrery/*:scripts/*`

**Planning files:** `Rocket Factory.md:docs/*`

**How to read this:** work goes strictly top to bottom through **Up Next**, one step at a time. The **#** column is the only order. Step IDs are kept because commits and notes cite them. A step whose "Waits on" names an Open Question or a **[USER]** action doesn't start until that's resolved.

**Who runs what (phased workflow + Orrery `gemini-phased`):**
- **Planner / Reviewer (Claude or review subagent)** writes the active phase (full step blocks to the zero-judgment bar), outlines future phases, and performs one phase review at the end of each phase using snapshot-backed step commits (`git show <sha>`).
- **Orchestrator / Validator (Hydrogen / Orrery `agy-hydrogen-orch` + `agy-hydrogen-val`)** runs the phase. At the start of each step, `phased:step_start` (`scripts/phased-step.sh pregate` + `neb start` + `scripts/phased-step.sh start`) takes a repository snapshot. After Neon finishes, `phased:step_check` (`scripts/phased-step.sh check`) validates scope and `POST_GATE` strictly against that snapshot diff, the validator reads the diff, and `phased:step_commit` commits the step and closes its Nebulous task.
- **Coder (Neon / Orrery `agy-neon-impl`)** executes one step inside `ALLOWED_FILES`. It never commits, never edits this plan or forbidden paths, and files bugs or scope lapses as **pending** Nebulous items.
- **Two code-fault failures** on one step, or an unamendable scope lapse → the step is deferred (`phased:step_defer`: saves attempt patch, restores step files, parks the task). A step whose `PRE_GATE` fails is deferred without dispatching an agent. Deferrals exceeding `max(2, ⌈steps/3⌉)` stop the phase early.

**Phase goal (Phase 1):** Establish the single-crate Rust foundation (`rocket-factory`), core domain model, atomic `.rocket/` TOML+MD storage, planning archetypes, coverage/gap audit engine, tool-agnostic canonical exporters, and RON CRT-amber theme loader — with zero references to any specific external tracker in `src/`.

**Side findings, bugs and scope lapses** become **pending neb items**, reviewed once at phase end. Every Up Next row has a neb task (`source_ref` = `STEP <ID>`); commits start with `[<TASK-ID>](<scope>)`.

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 1A** | `/goal` | core | `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/error.rs`, `src/model.rs` | — |
| 2 | **STEP 1B** | `/goal` | storage | `src/storage.rs`, `src/lib.rs` | STEP 1A |
| 3 | **STEP 1C** | `/goal` | archetypes | `src/archetype.rs`, `src/lib.rs` | STEP 1A |
| 4 | **STEP 1D** | `/goal` | audit | `src/audit.rs`, `src/lib.rs` | STEP 1A |
| 5 | **STEP 1E** | `/goal` | export | `src/export.rs`, `src/lib.rs` | STEP 1A |
| 6 | **STEP 1F** | `/goal` | theme | `src/theme.rs`, `src/lib.rs` | STEP 1A |

### Completed

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |

### Future phases

Outlines only (goal + step titles). Full step blocks are written at the phase review before each phase starts, followed by `neb plan import --write` and `scripts/phased-step.sh new-phase` (which regenerates `.orrery/phase.toml` for that phase).

- **Phase 2: Pluggable External Adapters & Headless CLI (`rf`)**
  - Adapter configuration schema and loader (`~/.config/rocket-factory/adapters.toml` + `.rocket/project.toml` bindings)
  - Subprocess adapter runner (`push_component_cmd`, `push_feature_cmd`, `pull_status_cmd`) using canonical JSON stdin/stdout contract
  - Task status reconciliation engine (merging pulled statuses into `.rocket/components/<id>.toml`)
  - Headless CLI (`src/main.rs`, `src/cli.rs` via `clap`): `rf init`, `rf audit`, `rf export`, `rf push`, `rf pull`, and default TUI launch entrypoint
- **Phase 3: TUI Shell, 3-Column Layout & Component List**
  - Terminal lifecycle manager (raw mode, alternate screen, panic restore hook, suspend/resume for `$EDITOR`)
  - Application state machine (`FocusColumn`, `InputMode`, filter/audit toggles, keyboard dispatcher)
  - Header bar widget and Left Column filterable component list with coverage gap indicators
  - Right Column component detail breakdown view (metadata, connections, feature checklist, themed Markdown preview)
- **Phase 4: Snap Grid Canvas & Orthogonal Vector Connectors**
  - Cell-based snap grid viewport (panning, cell coordinates, routing gutters)
  - Component card widget (name, status badge, progress `done/total`, brief description, themed status border)
  - Orthogonal Unicode box-drawing connector router (`─ │ ┌ ┐ └ ┘ ▶ ◀ ▲ ▼`) with directional arrows and typed edge labels
  - Keyboard-only canvas controls (`Navigate`, `MoveCard`, `Connect` modes with arrows and `hjkl` aliases)
- **Phase 5: Interactive Editing, Scaffolding Modals & `$EDITOR` Integration**
  - New Component archetype picker and scaffold modal
  - Structured TOML field & inline feature editor modal
  - Connection creation/editing modal (`EdgeKind` + optional label)
  - Suspend-to-`$EDITOR` workflow for `<id>.md` with automatic reload and audit refresh on exit
- **Phase 6: Live Sync Integration, Help Overlay & End-to-End Verification**
  - TUI startup pull, manual sync trigger (`r`/`S`), and optional background auto-poll interval
  - `?` Keyboard help overlay and RON theme switcher modal
  - End-to-end integration tests (CLI + mock adapter scripts + Ratatui `TestBackend` snapshot render tests)

### Open Questions

_None._

### Parked

Tracked in neb: `neb list --status pending`. Reviewed at phase end; only picked items become steps.

---

## Phase 1 Step Blocks

### STEP 1A: Cargo Manifest, Error Types, and Core Domain Model

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1A: Create Cargo.toml, src/lib.rs, src/error.rs, and src/model.rs defining the Rocket Factory domain types and unit tests.

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml (NEW)
  - Cargo.lock (NEW)
  - src/lib.rs (NEW)
  - src/error.rs (NEW)
  - src/model.rs (NEW)
READ_ONLY_REFS:
  - Rocket Factory.md:1-25 (requirements overview)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or Cargo.toml.
  - No TUI or CLI crates (ratatui, crossterm, clap) in Cargo.toml yet.

[PRE_GATE]
true

[MUTATION_SPEC]
1. Cargo.toml (NEW) — create package manifest:
   - `[package]`: `name = "rocket-factory"`, `version = "0.1.0"`, `edition = "2021"`, `description = "Retro-futuristic TUI development planning platform"`.
   - `[lib]`: `name = "rocket_factory"`, `path = "src/lib.rs"`.
   - `[dependencies]`: `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `toml = "0.8"`, `ron = "0.8"`, `thiserror = "2"`.
   - `[dev-dependencies]`: `tempfile = "3"`.
2. src/lib.rs (NEW) — declare modules and re-export error result:
   - `pub mod error;`
   - `pub mod model;`
   - `pub use error::{RfError, RfResult};`
3. src/error.rs (NEW) — define `RfError` using `#[derive(Debug, thiserror::Error)]` and `pub type RfResult<T> = std::result::Result<T, RfError>;` with variants:
   - `Io { path: std::path::PathBuf, #[source] source: std::io::Error }` (`"I/O error at {path}: {source}"`)
   - `InvalidComponentId(String)` (`"invalid component id: {0}"`)
   - `PathEscape(std::path::PathBuf)` (`"path escapes workspace boundary: {0}"`)
   - `TomlParse { path: std::path::PathBuf, #[source] source: toml::de::Error }` (`"TOML parse error in {path}: {source}"`)
   - `TomlSerialize(#[from] toml::ser::Error)` (`"TOML serialization error: {0}"`)
   - `JsonSerialize(#[from] serde_json::Error)` (`"JSON serialization error: {0}"`)
   - `RonParse { path: std::path::PathBuf, #[source] source: ron::error::SpannedError }` (`"RON parse error in {path}: {source}"`)
   - `NotFound(String)` (`"not found: {0}"`)
   - `AlreadyExists(String)` (`"already exists: {0}"`)
4. src/model.rs (NEW) — define serde (`Serialize, Deserialize`) domain types:
   - `pub struct GridPos { pub col: u16, pub row: u16 }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default`).
   - `pub enum ComponentKind { UiView, DataStream, ServiceCore, CliTool, Storage, Custom }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default` with `#[default] Custom` and `#[serde(rename_all = "snake_case")]`).
   - `pub enum EdgeKind { DependsOn, DataFlow, Renders, Invokes, Custom }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default` with `#[default] DependsOn` and `#[serde(rename_all = "snake_case")]`).
   - `pub enum TaskStatus { Pending, InProgress, Blocked, Complete }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default` with `#[default] Pending` and `#[serde(rename_all = "snake_case")]`).
   - `pub enum CardStatus { Gap, Planned, InProgress, Blocked, Complete }` (`Debug, Clone, Copy, PartialEq, Eq, Hash`).
   - `pub struct ExternalTaskRef { pub external_id: String, pub title: String, pub status: TaskStatus }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct FeatureItem { pub id: String, pub title: String, pub pushed: bool, pub external_id: Option<String> }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct Connection { pub from: String, pub to: String, pub kind: EdgeKind, pub label: Option<String> }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct Component { pub id: String, pub name: String, pub kind: ComponentKind, pub description: String, pub relative_path: Option<String>, pub position: GridPos, pub tags: Vec<String>, pub tech_stack: Vec<String>, pub files: Vec<String>, pub flags: Vec<String>, pub features: Vec<FeatureItem>, pub external_tasks: Vec<ExternalTaskRef>, #[serde(skip)] pub markdown_body: String }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct GridConfig { pub cols: u16, pub rows: u16 }` (`Debug, Clone, Copy, PartialEq, Eq`; `Default` returns `cols: 8, rows: 6`).
   - `pub struct ProjectConfig { pub name: String, pub description: String, pub theme: String, pub active_adapters: Vec<String>, pub grid: GridConfig, pub connections: Vec<Connection> }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct Project { pub config: ProjectConfig, pub components: Vec<Component> }` (`Debug, Clone, PartialEq, Eq`).
   - Helper method `impl Project { pub fn component(&self, id: &str) -> Option<&Component> }` and `pub fn component_mut(&mut self, id: &str) -> Option<&mut Component>`.
5. Tests in `src/model.rs`:
   - `model_toml_roundtrip`: construct a `ProjectConfig` with 1 `Connection` (`EdgeKind::DataFlow`, label `Some("events".into())`) and a `Component` with `ComponentKind::UiView`, 1 `FeatureItem`, and 1 `ExternalTaskRef` (`TaskStatus::Complete`); serialize both with `toml::to_string_pretty` and deserialize back; assert equality (`assert_eq!(parsed_cfg, cfg)` and `assert_eq!(parsed_comp, comp)` with `markdown_body` empty).
   - `project_component_lookup`: construct a `Project` with 2 components (`"ui_view"`, `"stream_core"`); assert `project.component("ui_view").is_some()` and `project.component("missing").is_none()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub struct Project\b' src/model.rs && grep -qE 'pub enum RfError\b' src/error.rs && ! grep -rqi 'nebulous' src/ Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1B: Atomic `.rocket/` Workspace Storage

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1B: Implement atomic, path-safe .rocket/ workspace persistence (TOML + Markdown) in src/storage.rs and expose it from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/storage.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:1-40 (RfError, RfResult)
  - src/model.rs:1-140 (Project, ProjectConfig, Component, GridConfig)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct Project\b' src/model.rs && grep -qE 'pub enum RfError\b' src/error.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod storage;` directly beneath it.
2. src/storage.rs (NEW) — define workspace constants and functions:
   - `pub const ROCKET_DIR: &str = ".rocket";`
   - `pub const PROJECT_FILE: &str = "project.toml";`
   - `pub const COMPONENTS_DIR: &str = "components";`
   - `pub fn validate_component_id(id: &str) -> RfResult<()>`: rejects empty strings, strings longer than 64 chars, or any char outside `a-z`, `0-9`, `_`, `-` (and rejects leading `-`), returning `Err(RfError::InvalidComponentId(id.to_string()))`.
   - `fn atomic_write(path: &std::path::Path, contents: &str) -> RfResult<()>`: writes to a sibling temporary file `<filename>.tmp.<pid>` in the same parent directory, flushes/syncs, and atomically renames to `path` via `std::fs::rename`.
   - `fn ensure_contained(base: &std::path::Path, candidate: &std::path::Path) -> RfResult<std::path::PathBuf>`: canonicalizes `base` and verifies the target parent canonicalizes within `base` (`starts_with`), returning `Err(RfError::PathEscape(candidate.to_path_buf()))` on escape.
   - `pub fn init_workspace(root: &std::path::Path, name: &str, description: &str) -> RfResult<Project>`: creates `<root>/.rocket/components/`, fails with `RfError::AlreadyExists` if `<root>/.rocket/project.toml` already exists, writes default `ProjectConfig` (`theme = "crt_amber"`, `active_adapters = []`, `grid = GridConfig::default()`, `connections = []`) atomically, and returns `Project`.
   - `pub fn save_project_config(root: &std::path::Path, config: &ProjectConfig) -> RfResult<()>`: validates containment and atomically writes `<root>/.rocket/project.toml`.
   - `pub fn save_component(root: &std::path::Path, component: &Component) -> RfResult<()>`: calls `validate_component_id(&component.id)`, verifies containment inside `<root>/.rocket/components/`, atomically writes `<id>.toml` and `<id>.md` (`component.markdown_body`).
   - `pub fn delete_component(root: &std::path::Path, id: &str) -> RfResult<()>`: validates id and containment, removes `<id>.toml` (returning `RfError::NotFound` if missing) and removes `<id>.md` if present.
   - `pub fn load_project(root: &std::path::Path) -> RfResult<Project>`: reads `<root>/.rocket/project.toml` (returning `RfError::NotFound` if missing), scans `<root>/.rocket/components/*.toml` sorted by filename, parses each `Component`, loads companion `<id>.md` if present (default empty string), and returns `Project`.
3. Tests in `src/storage.rs`:
   - `storage_init_save_load_roundtrip`: use `tempfile::tempdir()`; call `init_workspace(dir.path(), "demo-app", "Demo")`; save a component `"view_box"` with `markdown_body = "# View Box\nDetails\n".to_string()`; call `load_project(dir.path())`; assert `loaded.config.name == "demo-app"`, `loaded.components.len() == 1`, `loaded.components[0].id == "view_box"`, and `loaded.components[0].markdown_body == "# View Box\nDetails\n"`.
   - `storage_rejects_invalid_and_traversal_ids`: assert `validate_component_id("../escape").is_err()`, `validate_component_id("UpperCase").is_err()`, `validate_component_id("").is_err()`, and `validate_component_id("valid_id-1").is_ok()`.
   - `storage_delete_component_removes_both_files`: init workspace, save `"stream_1"`, call `delete_component(dir.path(), "stream_1")`, assert `load_project(dir.path()).unwrap().components.is_empty()` and assert deleting again returns `Err(RfError::NotFound(_))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn load_project\b' src/storage.rs && grep -qE 'pub fn save_component\b' src/storage.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1C: Planning-First Component Archetypes

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1C: Implement planning-first component archetype templates in src/archetype.rs and expose them from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/archetype.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/model.rs:1-140 (Component, ComponentKind, FeatureItem, GridPos)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No filesystem I/O inside src/archetype.rs (pure data constructors only).

[PRE_GATE]
grep -qE 'pub enum ComponentKind\b' src/model.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod archetype;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/archetype.rs (NEW) — define archetype descriptor and scaffold builder:
   - `pub struct ArchetypeSpec { pub kind: ComponentKind, pub label: &'static str, pub summary: &'static str, pub default_tags: &'static [&'static str], pub starter_features: &'static [&'static str], pub markdown_template: &'static str }` (`Debug, Clone, Copy`).
   - `pub const ARCHETYPES: &[ArchetypeSpec]` containing exactly 6 entries covering every `ComponentKind` variant:
     1. `ComponentKind::UiView` (`label: "UI View"`, `default_tags: &["ui", "view"]`, `starter_features: &["Define layout and container bounds", "Wire state bindings and input handling"]`)
     2. `ComponentKind::DataStream` (`label: "Data Stream"`, `default_tags: &["data", "stream"]`, `starter_features: &["Define payload schema and framing", "Implement producer/consumer pipeline"]`)
     3. `ComponentKind::ServiceCore` (`label: "Service Core"`, `default_tags: &["core", "service"]`, `starter_features: &["Define domain API and error contracts", "Implement core business logic"]`)
     4. `ComponentKind::CliTool` (`label: "CLI Tool"`, `default_tags: &["cli"]`, `starter_features: &["Define argument parser and subcommands", "Implement exit codes and output formatting"]`)
     5. `ComponentKind::Storage` (`label: "Storage"`, `default_tags: &["storage", "persistence"]`, `starter_features: &["Define persistence schema and migrations", "Implement atomic read/write layer"]`)
     6. `ComponentKind::Custom` (`label: "Custom"`, `default_tags: &["custom"]`, `starter_features: &["Define component responsibilities"]`)
   - `pub fn archetype_for(kind: ComponentKind) -> &'static ArchetypeSpec`: matches `kind` to its entry in `ARCHETYPES`.
   - `pub fn scaffold_component(id: &str, name: &str, kind: ComponentKind, position: GridPos) -> Component`: builds a `Component` with `id`, `name`, `kind`, `description` set to `spec.summary`, `position`, `tags` cloned from `spec.default_tags`, `features` generated as `FeatureItem { id: format!("{id}_f{}", idx + 1), title: (*title).to_string(), pushed: false, external_id: None }`, `external_tasks: Vec::new()`, and `markdown_body` formatted as `format!("# {name}\n\n{}", spec.markdown_template)`.
3. Tests in `src/archetype.rs`:
   - `archetype_covers_all_kinds`: iterate over `[ComponentKind::UiView, ComponentKind::DataStream, ComponentKind::ServiceCore, ComponentKind::CliTool, ComponentKind::Storage, ComponentKind::Custom]`; assert `archetype_for(kind).kind == kind` and `!archetype_for(kind).starter_features.is_empty()`.
   - `scaffold_ui_view_populates_features_and_markdown`: call `scaffold_component("main_view", "Main View", ComponentKind::UiView, GridPos { col: 2, row: 1 })`; assert `comp.id == "main_view"`, `comp.features.len() == 2`, `comp.features[0].id == "main_view_f1"`, `comp.markdown_body.starts_with("# Main View\n\n")`, and `comp.tags.contains(&"ui".to_string())`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn scaffold_component\b' src/archetype.rs && grep -qE 'pub const ARCHETYPES\b' src/archetype.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1D: Coverage Audit and Card Status Engine

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1D: Implement card status computation and overlooked-component coverage auditing in src/audit.rs and expose it from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/audit.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/model.rs:1-140 (Project, Component, CardStatus, TaskStatus, Connection, FeatureItem, ExternalTaskRef)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub enum CardStatus\b' src/model.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod audit;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/audit.rs (NEW) — define status computation, audit finding types, and project audit engine:
   - `pub enum FindingSeverity { Warning, Error }` (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub enum FindingKind { UntrackedComponent, UnpushedFeatures, DanglingConnection }` (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub struct AuditFinding { pub severity: FindingSeverity, pub kind: FindingKind, pub component_id: Option<String>, pub message: String }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct TaskProgress { pub complete: usize, pub total: usize }` (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub fn compute_card_status(component: &Component) -> CardStatus`:
     - If `component.external_tasks.is_empty()` and `component.features.is_empty()` → `CardStatus::Gap`.
     - If `component.external_tasks.is_empty()` and `!component.features.is_empty()` → `CardStatus::Planned`.
     - If any task in `component.external_tasks` has `TaskStatus::Blocked` → `CardStatus::Blocked`.
     - If all tasks in `component.external_tasks` have `TaskStatus::Complete` → `CardStatus::Complete`.
     - Otherwise → `CardStatus::InProgress`.
   - `pub fn task_progress(component: &Component) -> TaskProgress`: counts `complete` (`status == TaskStatus::Complete`) and `total` (`external_tasks.len()`).
   - `pub fn audit_project(project: &Project) -> Vec<AuditFinding>`:
     - Collects valid component IDs into a `HashSet<&str>`.
     - For each component:
       - If `component.external_tasks.is_empty() && component.features.is_empty()`: push `AuditFinding` with `FindingSeverity::Error`, `FindingKind::UntrackedComponent`, `component_id: Some(component.id.clone())`, `message: format!("Component '{}' has no features or linked tasks", component.id)`.
       - Else if `component.external_tasks.is_empty()` and any feature has `!feature.pushed`: push `AuditFinding` with `FindingSeverity::Warning`, `FindingKind::UnpushedFeatures`, `component_id: Some(component.id.clone())`, `message: format!("Component '{}' has unpushed local features and no linked tasks", component.id)`.
       - Else if any feature has `!feature.pushed`: push `AuditFinding` with `FindingSeverity::Warning`, `FindingKind::UnpushedFeatures`, `component_id: Some(component.id.clone())`, `message: format!("Component '{}' has {} unpushed local feature(s)", component.id, unpushed_count)`.
     - For each connection in `project.config.connections`:
       - If `!ids.contains(conn.from.as_str()) || !ids.contains(conn.to.as_str())`: push `AuditFinding` with `FindingSeverity::Error`, `FindingKind::DanglingConnection`, `component_id: None`, `message: format!("Connection '{} -> {}' references missing component", conn.from, conn.to)`.
3. Tests in `src/audit.rs`:
   - `card_status_transitions`: verify all 5 `CardStatus` outcomes: empty component -> `Gap`; local feature only -> `Planned`; mixed `Pending`/`Complete` external tasks -> `InProgress`; any `Blocked` task -> `Blocked`; all `Complete` external tasks -> `Complete`.
   - `audit_flags_untracked_view_box_and_dangling_edge`: build a `Project` with `"data_stream"` (1 `Complete` external task) and `"view_box"` (0 features, 0 external tasks), plus a connection `"data_stream" -> "ghost_node"`; call `audit_project(&project)`; assert findings contain an `UntrackedComponent` for `"view_box"` and a `DanglingConnection` for `"ghost_node"`.
   - `task_progress_counts_completed_tasks`: build component with 2 `Complete` and 1 `InProgress` external tasks; assert `task_progress(&comp) == TaskProgress { complete: 2, total: 3 }`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn compute_card_status\b' src/audit.rs && grep -qE 'pub fn audit_project\b' src/audit.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1E: Tool-Agnostic Canonical Exporters (TOML Manifest, Roadmap MD, JSON)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1E: Implement tool-agnostic TOML manifest, Markdown roadmap table, and canonical JSON exporters in src/export.rs and expose them from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/export.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:1-40 (RfError, RfResult)
  - src/model.rs:1-140 (Project, Component, ComponentKind, Connection, FeatureItem)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct Project\b' src/model.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod export;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/export.rs (NEW) — implement three canonical exporters:
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] pub struct ExportDependency { pub target: String, pub kind: EdgeKind, pub label: Option<String> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] pub struct ComponentManifestExport { pub id: String, pub name: String, pub component_type: String, pub relative_path: Option<String>, pub description: String, pub tags: Vec<String>, pub tech_stack: Vec<String>, pub files: Vec<String>, pub flags: Vec<String>, pub dependencies: Vec<ExportDependency> }`
   - `pub fn map_component_type(kind: ComponentKind) -> &'static str`: maps `UiView => "ui_crate"`, `DataStream => "rust_crate"`, `ServiceCore => "rust_crate"`, `CliTool => "cli_crate"`, `Storage => "rust_crate"`, `Custom => "custom"`.
   - `pub fn export_component_manifest_toml(project: &Project, component_id: &str) -> RfResult<String>`: looks up `component_id` (or `Err(RfError::NotFound(...))`), gathers outgoing `project.config.connections` where `conn.from == component_id` into `dependencies`, builds `ComponentManifestExport`, and serializes with `toml::to_string_pretty`.
   - `pub fn export_roadmap_markdown(project: &Project) -> String`: generates a Markdown document with `# <project.config.name> — Blueprint Roadmap`, an `## Up Next` table with columns `| # | Source Ref | Component | Feature | Status |` listing every component's features (1-based index, `Source Ref` = `FEAT <component.id>:<feature.id>`, `Status` = `"pushed"` if `feature.pushed` else `"planned"`), and a `## Components Without Tasks` section listing any component with `features.is_empty() && external_tasks.is_empty()`.
   - `pub fn export_project_json(project: &Project) -> RfResult<String>`: serializes `project` with `serde_json::to_string_pretty`.
3. Tests in `src/export.rs`:
   - `export_manifest_toml_includes_dependencies_and_type`: build a `Project` with `"ui_box"` (`ComponentKind::UiView`), `"core_bus"` (`ComponentKind::DataStream`), and connection `"ui_box" -> "core_bus"` (`EdgeKind::DataFlow`); call `export_component_manifest_toml(&project, "ui_box").unwrap()`; assert output contains `component_type = "ui_crate"`, `id = "ui_box"`, and `target = "core_bus"`.
   - `export_roadmap_markdown_lists_features_and_gaps`: build a `Project` with `"core_bus"` (1 unpushed feature `"f1": "Stream events"`) and `"view_box"` (0 features, 0 external tasks); call `export_roadmap_markdown(&project)`; assert output contains `FEAT core_bus:f1`, `Stream events`, and `view_box` under `## Components Without Tasks`.
   - `export_project_json_valid_json`: call `export_project_json(&project).unwrap()`; parse with `serde_json::from_str::<serde_json::Value>` and assert `val["config"]["name"] == "demo"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn export_component_manifest_toml\b' src/export.rs && grep -qE 'pub fn export_roadmap_markdown\b' src/export.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1F: RON Theme System and Default CRT Amber Palette

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1F: Implement RON-based theme loading and bundled CRT amber-on-black default palette in src/theme.rs and expose it from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/theme.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:1-40 (RfError, RfResult)
  - Rocket Factory.md:20-25 (aesthetics and CRT amber theme requirements)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No ratatui dependency in src/theme.rs (use plain RGB tuples/structs so core stays UI-library-agnostic).
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'ron =' Cargo.toml

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod theme;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/theme.rs (NEW) — define RGB color and RON theme types + loader:
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct RgbColor { pub r: u8, pub g: u8, pub b: u8 }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct ThemePalette`:
     - `pub background: RgbColor`
     - `pub surface: RgbColor`
     - `pub foreground: RgbColor`
     - `pub muted: RgbColor`
     - `pub accent: RgbColor`
     - `pub border_default: RgbColor`
     - `pub border_focused: RgbColor`
     - `pub status_gap: RgbColor`
     - `pub status_planned: RgbColor`
     - `pub status_in_progress: RgbColor`
     - `pub status_blocked: RgbColor`
     - `pub status_complete: RgbColor`
     - `pub md_heading: RgbColor`
     - `pub md_code: RgbColor`
     - `pub alert_warn: RgbColor`
     - `pub alert_error: RgbColor`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct Theme { pub name: String, pub palette: ThemePalette }`
   - `pub const DEFAULT_CRT_AMBER_RON: &str`: embedded RON string for `"crt_amber"` with pure CRT black background `(10, 10, 8)`, phosphor amber foreground `(255, 176, 0)`, bright amber focus `(255, 209, 102)`, phosphor green complete border `(64, 224, 112)`, alert red blocked/error `(255, 82, 82)`, cyan/teal code highlight `(94, 212, 212)`, and differentiated amber/gold heading & warning tones.
   - `pub fn default_theme() -> RfResult<Theme>`: parses `DEFAULT_CRT_AMBER_RON` via `ron::from_str`.
   - `pub fn load_theme_file(path: &std::path::Path) -> RfResult<Theme>`: reads file and parses RON, returning `RfError::Io` or `RfError::RonParse`.
   - `pub fn resolve_theme(theme_name: &str, search_dirs: &[std::path::PathBuf]) -> RfResult<Theme>`: if `<dir>/<theme_name>.ron` exists in any `search_dirs` (checked in order), loads it via `load_theme_file`; if `theme_name == "crt_amber"` and no override file exists, returns `default_theme()`; otherwise returns `Err(RfError::NotFound(format!("theme '{theme_name}'")))`.
3. Tests in `src/theme.rs`:
   - `default_crt_amber_theme_parses_with_green_complete_border`: call `let t = default_theme().unwrap();`; assert `t.name == "crt_amber"`, `t.palette.background == RgbColor { r: 10, g: 10, b: 8 }`, `t.palette.foreground == RgbColor { r: 255, g: 176, b: 0 }`, and `t.palette.status_complete.g > t.palette.status_complete.r`.
   - `resolve_theme_loads_override_file`: create `tempfile::tempdir()`, write a custom `ocean.ron` theme file, call `resolve_theme("ocean", &[dir.path().to_path_buf()]).unwrap()`, assert `theme.name == "ocean"`; also assert `resolve_theme("nonexistent", &[dir.path().to_path_buf()]).is_err()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn resolve_theme\b' src/theme.rs && grep -qE 'pub const DEFAULT_CRT_AMBER_RON\b' src/theme.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```
