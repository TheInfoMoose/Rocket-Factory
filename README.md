# Rocket Bench (`rktb`)

Rocket Bench is a Rust terminal user interface (`ratatui`) and CLI tool for planning application architectures using `.rocket/` TOML metadata files, Markdown component specifications, and external command-based adapters.

## Prerequisites

- **Rust** (stable toolchain, installed via [rustup](https://rustup.rs/))
- **Git** (`git` CLI)

## Installation

### Install directly from Git (recommended)

```bash
cargo install --git https://github.com/TheInfoMoose/Rocket-Factory.git --locked
```

### Build and install from a local clone

```bash
git clone https://github.com/TheInfoMoose/Rocket-Factory.git
cd Rocket-Factory

# Install `rktb` into ~/.cargo/bin
cargo install --path . --locked
```

#### Build a local release binary without installing

To build `target/release/rktb` without installing into `~/.cargo/bin`:

```bash
cargo build --release --locked
```

### Verify installation & PATH

`cargo install` places the `rktb` binary in `~/.cargo/bin`. Ensure `$HOME/.cargo/bin` is on your `PATH`, then verify the installation:

```bash
rktb --help
```

## Quick Start

1. **Initialize a workspace** (creates `./my-project/.rocket/` automatically):

   ```bash
   rktb --path ./my-project init --name "my-project" --description "Service architecture blueprint"
   ```

   Or initialize inside an existing project directory (`--path` defaults to `.`):

   ```bash
   rktb init --name "my-project"
   ```

2. **Launch the interactive TUI** (requires an initialized `.rocket/` workspace — run `rktb init` first):

   ```bash
   rktb --path ./my-project
   ```

   Or run `rktb` directly inside the workspace directory.

3. **Explore the TUI & verify via CLI**:

   - Press `n` to add a component, `?` for keyboard help, and `q` to quit.
   - Optionally run a CLI audit on the workspace:

     ```bash
     rktb --path ./my-project audit
     ```

## CLI Reference

Running `rktb` without a subcommand opens the interactive TUI.

### Global Flags

| Flag | Default | Description |
| :--- | :--- | :--- |
| `--path <DIR>` | `.` | Workspace root directory containing `.rocket/`. |
| `--adapters-file <PATH>` | `~/.config/rocket-bench/adapters.toml` | Path to the external adapters configuration file. Defaults to `~/.config/rocket-bench/adapters.toml` when `$HOME` is set and non-empty; when `$HOME` is unset or empty, no implicit workspace `.rocket/adapters.toml` fallback is used (pass `--adapters-file` explicitly). |
| `--auto-pull` | `false` | Boolean flag that enables a one-shot pull on TUI startup when `active_adapters` are configured. |
| `--version` / `-V` | — | Prints `rktb <version>` and exits. |

### Subcommands

| Command | Description |
| :--- | :--- |
| `rktb init --name <NAME> [--description <DESC>]` | Initializes `.rocket/project.toml` and `.rocket/components/` in the workspace root. Fails if `.rocket/project.toml` already exists. `--description` defaults to `""`. |
| `rktb audit [--strict]` | Audits the workspace and prints `[ERROR]` and `[WARN]` findings (or `No audit findings.`). With `--strict`, exits with status `1` if any `[ERROR]` finding is present. |
| `rktb export --format <manifest\|roadmap\|json> [--component <ID>]` | Exports workspace data to stdout: `manifest` exports a single component's TOML manifest (requires `--component <ID>`), `roadmap` exports a Markdown roadmap, and `json` exports the full project as JSON. |
| `rktb push [--component <ID>] [--adapter <NAME>]` | Pushes component(s) via `push_component_cmd` and unpushed features via `push_feature_cmd`. Targets a single component if `--component` is given, otherwise all components. Uses `--adapter` if given, otherwise `active_adapters` from `.rocket/project.toml`. Persists updated `pushed` and `external_id` fields on features. |
| `rktb pull [--adapter <NAME>]` | Pulls external task status via `pull_status_cmd` from `--adapter` or all `active_adapters`, reconciles `external_tasks` and feature links, and saves updated component files. |

### Environment Variables

| Variable | Behavior |
| :--- | :--- |
| `RKTB_POLL_INTERVAL_SECS` | Positive integer interval in seconds for automatic `pull` sync in the TUI when `active_adapters` is non-empty and no modal is open. Disabled when unset, invalid, or `0`. |
| `EDITOR` | External editor command launched when pressing `v` in the TUI to edit `.rocket/components/<id>.md`. Split on ASCII whitespace into program and arguments; falls back to `vi` when unset or blank. |

## TUI Keyboard Reference

The TUI layout consists of a header bar, three columns (Component List, Snap Grid Canvas, and Component Details), and a footer bar.

### Navigation

| Key | Action |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Cycle focus (List / Canvas / Detail) |
| `hjkl` / Arrows | Move selection / Scroll detail |
| `Space` / `Enter` | Expand or collapse sub-components of the selected parent card (with automatic collision shifting on expand) |
| `f` | Toggle Gaps Only filter |

### Editing & Canvas

| Key | Action |
| :--- | :--- |
| `n` | New top-level component (defaults to top-center placement on the canvas) |
| `a` | Add a sub-component under the selected component (Navigate mode) |
| `e` | Edit selected component |
| `E` | Edit outgoing edge from selected component (opens with the first matching connection selected when multiple exist) |
| `d` | Delete selected component |
| `D` | Delete connection on selected component (opens with the first matching connection selected when multiple exist) |
| `m` | Toggle Move Card mode |
| `c` | Connect components (`<A> depends on <target>`, i.e. `from = <A>`, `to = <target>`); confirming on an already-connected target toggles and removes the existing edge |
| `Enter` (Connect mode) | Confirm connect target |
| `v` | Open component Markdown in `$EDITOR` |

### Sync & Display

| Key | Action |
| :--- | :--- |
| `+` / `=` | Zoom in (`Compact` → `Normal` → `Expanded`; defaults to `Compact`, fitting `>= 4×4` cards on `>= 114x26` terminals) |
| `-` | Zoom out (`Expanded` → `Normal` → `Compact`) |
| `r` / `S` | Pull status from active adapters |
| `p` | Push selected component & features |
| `t` | Switch RON theme |

### General

| Key | Action |
| :--- | :--- |
| `?` | Toggle this keyboard help |
| `Esc` | Cancel current mode |
| `q` | Quit |
| `Ctrl+C` | Force quit |

### Modal Controls

- **Component / Sub-Component / Connection forms (`n`, `a`, `e`, `E`)**: `Tab`, `Shift+Tab`, `↑`, and `↓` navigate between fields, `←`/`→` cycles enum options, `Enter` saves, `Esc` cancels.
  - `a` (in Navigate mode) opens the New Sub-Component modal scoped under the selected parent component.
  - `e` (Edit Component) supports changing component archetypes (`kind`) and includes an `Add Sub-component` action.
  - `E` (Edit Edge) supports `EdgeKind::Contains` (`contains`) to ingest existing cards as sub-components or detach them when changed away. When multiple matching connections exist, the modal opens with the first matching connection selected: cycle candidates with `[` / `]` or `PgUp` / `PgDn` on the Kind field, and with `PgUp` / `PgDn` on the Label field (where `[` / `]` type literal characters).
- **Delete confirmation (`d`, `D`)**: `Enter` or `y` confirms deletion, `Esc` or `n` cancels. When `D` matches multiple connections, it opens with the first matching connection selected and `↑` / `↓` selects among candidates.
- **Theme switcher (`t`)**: `↑`/`↓` or `j`/`k` selects a theme, `Enter` applies and saves to `.rocket/project.toml`, `Esc` cancels.
- **Keyboard help (`?`)**: `Esc`, `?`, `Enter`, or `q` closes the modal.

## Workspace Layout

```text
<workspace>/
└── .rocket/
    ├── project.toml
    ├── components/
    │   ├── <id>.toml
    │   └── <id>.md
    └── themes/
        └── <theme_name>.ron
```

- **`.rocket/project.toml`**: Stores `ProjectConfig` — `name`, `description`, `theme` (default `"crt_amber"`), `active_adapters` (list of adapter names from `adapters.toml`), `grid` (`cols` and `rows`, default `8` × `6`; the canvas uses an effective grid floor of at least the visible canvas capacity minus sidebars, and `project.toml` automatically expands when a card is placed or moved beyond the saved bounds), and `connections` (`from`, `to`, `kind`: `depends_on` | `data_flow` | `renders` | `invokes` | `contains` | `custom`, optional `label`).
- **`.rocket/components/<id>.toml`**: Stores `Component` metadata — `id` (1–64 characters from `[a-z0-9_-]`, cannot start with `-`), `name`, `kind` (lightweight prototyping archetypes: `concept` default | `interface` | `logic` | `data` | `integration` | `custom`; backwards-compatible with legacy `kind` values `ui_view` | `data_stream` | `service_core` | `cli_tool` | `storage`), optional `parent_id` (parent component ID for sub-components), `description`, optional `relative_path`, `position` (`col`, `row`), `tags`, `tech_stack`, `files`, `flags`, `features` (`id`, `title`, `pushed`, `external_id`), and `external_tasks` (`external_id`, `title`, `status`: `pending` | `in_progress` | `blocked` | `complete`).
- **`.rocket/components/<id>.md`**: Markdown specification body for component `<id>`, rendered in the Component Details panel and edited with `v`.
- **`.rocket/themes/*.ron`**: Optional workspace-local RON theme files.

> **Note:** Hand-edited `.rocket/components/<id>.toml` files must include all required non-defaulted fields: `id`, `name`, `description`, `position`, `tags`, `tech_stack`, `files`, `flags`, `features`, and `external_tasks`.

## RON Themes

### Theme Search Order

When loading a theme named `<name>`, Rocket Bench searches in the following order:

1. `<workspace>/.rocket/themes/<name>.ron`
2. `~/.config/rocket-bench/themes/<name>.ron` (skipped when `$HOME` is unset or empty)
3. Built-in `crt_amber` (fallback when `<name>` is `"crt_amber"` and no override file exists)

Theme names may only contain ASCII alphanumerics, `_`, and `-`. Symlinks resolving outside the theme directory are rejected.

### Custom `.ron` Theme Example

Create `.rocket/themes/custom_amber.ron` (or `~/.config/rocket-bench/themes/custom_amber.ron`) with all 16 RGB palette fields:

```ron
(
    name: "custom_amber",
    palette: (
        background: (r: 10, g: 10, b: 8),
        surface: (r: 20, g: 18, b: 14),
        foreground: (r: 255, g: 176, b: 0),
        muted: (r: 153, g: 107, b: 20),
        accent: (r: 255, g: 196, b: 61),
        border_default: (r: 120, g: 82, b: 16),
        border_focused: (r: 255, g: 209, b: 102),
        status_gap: (r: 140, g: 100, b: 36),
        status_planned: (r: 214, g: 158, b: 46),
        status_in_progress: (r: 255, g: 196, b: 61),
        status_blocked: (r: 255, g: 82, b: 82),
        status_complete: (r: 64, g: 224, b: 112),
        md_heading: (r: 255, g: 218, b: 121),
        md_code: (r: 94, g: 212, b: 212),
        alert_warn: (r: 255, g: 159, b: 28),
        alert_error: (r: 255, g: 82, b: 82),
    ),
)
```

## External Adapters (`adapters.toml`)

Rocket Bench integrates with external tools by spawning configured commands and exchanging payloads over `stdin` and `stdout`.

### Configuration Path and Schema

By default, adapters are loaded from `~/.config/rocket-bench/adapters.toml` when `$HOME` is set and non-empty (overridable via `--adapters-file <PATH>`; when `$HOME` is unset or empty, pass `--adapters-file` explicitly). Enable adapters for a workspace by listing their `name` values under `active_adapters` in `.rocket/project.toml`.

```toml
[[adapters]]
name = "example_tracker"
export_format = "json" # "toml_manifest" | "markdown_roadmap" | "json" (default: "json")

[adapters.push_component_cmd]
program = "/usr/local/bin/example-tracker-adapter"
args = ["push-component"]

[adapters.push_feature_cmd]
program = "/usr/local/bin/example-tracker-adapter"
args = ["push-feature"]

[adapters.pull_status_cmd]
program = "/usr/local/bin/example-tracker-adapter"
args = ["pull-status"]
```

Each command is executed with the workspace root as its working directory. Non-zero exit codes fail the operation and surface `stderr`.

### Command Stdin/Stdout Contracts

#### `push_component_cmd`

- **stdin**: Formatted according to the adapter's `export_format`:
  - `toml_manifest`: Single component TOML manifest (same as `rktb export --format manifest --component <id>`).
  - `markdown_roadmap`: Full project Markdown roadmap (same as `rktb export --format roadmap`).
  - `json`: Pretty-printed JSON representation of the target `Component`.
- **stdout**: Empty output, or a `PushResponse` JSON object (both fields optional):

```json
{
  "external_id": "CMP-101",
  "message": "synced"
}
```

Any returned `external_id` and `message` values from `push_component_cmd` are surfaced in the push status summary and are not persisted onto the component.

#### `push_feature_cmd`

- **stdin**: `PushFeaturePayload` JSON object:

```json
{
  "component_id": "ui_view",
  "feature": {
    "id": "ui_view_f1",
    "title": "Render status header",
    "pushed": false,
    "external_id": null
  }
}
```

- **stdout**: Empty output, or a `PushResponse` JSON object:

```json
{
  "external_id": "TASK-204",
  "message": "created"
}
```

When multiple active adapters define `push_feature_cmd`, each unpushed feature is sent to every such adapter; if all succeed, the feature is marked `pushed = true` and records the first non-null `external_id` returned.

#### `pull_status_cmd`

- **stdin**: Full `Project` serialized as JSON (same as `rktb export --format json`).
- **stdout**: `PullStatusResponse` JSON object:

```json
{
  "components": [
    {
      "component_id": "ui_view",
      "tasks": [
        {
          "external_id": "TASK-204",
          "title": "Render status header",
          "status": "complete"
        }
      ]
    }
  ]
}
```

Valid task `status` values are `"pending"`, `"in_progress"`, `"blocked"`, and `"complete"`. During reconciliation, each matched component's `external_tasks` are replaced with the returned `tasks`, and features are linked (`pushed = true`, `external_id` populated) by matching `external_id` or exact `title`.

## Testing

Run the workspace formatting, lint, test, and documentation checks with:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
```

## Releases

Pushing a `v*` Git tag (or dispatching the Release workflow manually) triggers `.github/workflows/release.yml`, which builds release binaries and packages platform archives alongside `.sha256` checksum files for:

- **Linux** (`x86_64-unknown-linux-gnu`, `.tar.gz` + `.sha256`)
- **macOS** (`aarch64-apple-darwin`, `.tar.gz` + `.sha256`)
- **Windows** (`x86_64-pc-windows-msvc`, `.zip` + `.sha256`)

## Repository Layout

In addition to the Rust crate (`src/`, `tests/`, `Cargo.toml`), `.tracker/`, `.orrery/`, `.phased/`, `scripts/`, and `STEPPED_IMPLEMENTATION_PLAN.md` are tracked on `master` for project planning and workflow automation. These planning and workflow directories/files are excluded from published crate packages via the `exclude` list in `Cargo.toml`.

## License

Rocket Bench is licensed under the MIT License. See [LICENSE](LICENSE) for details.
