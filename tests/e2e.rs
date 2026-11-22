use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;
use rocket_factory::adapter::{AdapterConfig, AdaptersFile, CommandSpec, ExportFormat};
use rocket_factory::archetype::scaffold_component;
use rocket_factory::cli::{execute_command, Command, ExportKind};
use rocket_factory::model::{ComponentKind, Connection, EdgeKind, GridPos, TaskStatus};
use rocket_factory::storage;
use rocket_factory::theme::{default_theme, DEFAULT_CRT_AMBER_RON};
use rocket_factory::tui::app::App;
use rocket_factory::tui::ui::render;

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

fn render_app(app: &mut App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("create test terminal");
    terminal
        .draw(|frame| render(frame, app))
        .expect("draw frame");
    terminal.backend().buffer().clone()
}

fn write_mock_adapters_toml(adapters_path: &Path, pull_json: &str) {
    let adapters_file = AdaptersFile {
        adapters: vec![AdapterConfig {
            name: "mock_tracker".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: Some(CommandSpec {
                program: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    r#"cat >/dev/null; printf '{"external_id":"COMP-1","message":"pushed"}'"#
                        .to_string(),
                ],
            }),
            push_feature_cmd: Some(CommandSpec {
                program: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    r#"cat >/dev/null; printf '{"external_id":"EXT-101","message":"created"}'"#
                        .to_string(),
                ],
            }),
            pull_status_cmd: Some(CommandSpec {
                program: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    format!("cat >/dev/null; printf '%s' '{pull_json}'"),
                ],
            }),
        }],
    };
    let toml_content = toml::to_string_pretty(&adapters_file).expect("serialize adapters.toml");
    fs::write(adapters_path, toml_content).expect("write mock adapters.toml");
}

#[test]
fn e2e_cli_lifecycle_and_mock_adapter_roundtrip() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let adapters_path = dir.path().join("adapters.toml");

    // 1. Init workspace via CLI
    let init_out = execute_command(
        dir.path(),
        &adapters_path,
        Command::Init {
            name: "apollo-e2e".to_string(),
            description: "End-to-end plan for testuser@example.com".to_string(),
        },
    )
    .expect("init workspace");
    assert_eq!(init_out.exit_code, 0);
    assert!(init_out.stdout.contains("apollo-e2e"));

    // 2. Scaffold and save components (`view_box` with an unpushed feature, `data_stream` as a gap)
    let mut view_box = scaffold_component(
        "view_box",
        "View Box",
        ComponentKind::UiView,
        GridPos { col: 0, row: 0 },
    );
    view_box.features.truncate(1);
    view_box.relative_path = Some("src/view_box.rs".to_string());
    storage::save_component(dir.path(), &view_box).expect("save view_box");

    let mut data_stream = scaffold_component(
        "data_stream",
        "Data Stream",
        ComponentKind::DataStream,
        GridPos { col: 1, row: 0 },
    );
    data_stream.features.clear();
    data_stream.relative_path = Some("src/data_stream.rs".to_string());
    storage::save_component(dir.path(), &data_stream).expect("save data_stream");

    let mut project = storage::load_project(dir.path()).expect("load project");
    project.config.active_adapters = vec!["mock_tracker".to_string()];
    project.config.connections.push(Connection {
        from: "view_box".to_string(),
        to: "data_stream".to_string(),
        kind: EdgeKind::DataFlow,
        label: Some("telemetry".to_string()),
    });
    storage::save_project_config(dir.path(), &project.config).expect("save project config");

    // 3. Audit --strict exits non-zero while `data_stream` is a gap
    let audit_before = execute_command(dir.path(), &adapters_path, Command::Audit { strict: true })
        .expect("run strict audit before sync");
    assert_eq!(audit_before.exit_code, 1);
    assert!(audit_before.stdout.contains("data_stream"));

    // 4. Export Manifest, Roadmap, and Json formats
    let manifest_out = execute_command(
        dir.path(),
        &adapters_path,
        Command::Export {
            format: ExportKind::Manifest,
            component: Some("view_box".to_string()),
        },
    )
    .expect("export manifest");
    assert_eq!(manifest_out.exit_code, 0);
    assert!(manifest_out.stdout.contains("id = \"view_box\""));
    assert!(manifest_out
        .stdout
        .contains("component_type = \"ui_crate\""));
    assert!(manifest_out.stdout.contains("target = \"data_stream\""));

    let roadmap_out = execute_command(
        dir.path(),
        &adapters_path,
        Command::Export {
            format: ExportKind::Roadmap,
            component: None,
        },
    )
    .expect("export roadmap");
    assert_eq!(roadmap_out.exit_code, 0);
    assert!(roadmap_out
        .stdout
        .contains("# apollo-e2e — Blueprint Roadmap"));
    assert!(roadmap_out.stdout.contains("View Box"));
    assert!(roadmap_out.stdout.contains("Data Stream"));

    let json_out = execute_command(
        dir.path(),
        &adapters_path,
        Command::Export {
            format: ExportKind::Json,
            component: None,
        },
    )
    .expect("export json");
    assert_eq!(json_out.exit_code, 0);
    let parsed_json: serde_json::Value =
        serde_json::from_str(&json_out.stdout).expect("parse exported json");
    assert_eq!(parsed_json["config"]["name"], "apollo-e2e");
    assert_eq!(
        parsed_json["components"]
            .as_array()
            .expect("components array")
            .len(),
        2
    );
    assert_eq!(
        parsed_json["config"]["connections"]
            .as_array()
            .expect("connections array")
            .len(),
        1
    );

    // 5. Configure mock `sh` adapter and run Push -> Pull
    let pull_json = r#"{"components":[{"component_id":"view_box","tasks":[{"external_id":"EXT-101","title":"Define layout and container bounds","status":"complete"}]},{"component_id":"data_stream","tasks":[{"external_id":"EXT-202","title":"Ingest telemetry from 192.0.2.10","status":"complete"}]}]}"#;
    write_mock_adapters_toml(&adapters_path, pull_json);

    let push_out = execute_command(
        dir.path(),
        &adapters_path,
        Command::Push {
            component: None,
            adapter: None,
        },
    )
    .expect("run push");
    assert_eq!(push_out.exit_code, 0);
    assert!(push_out.stdout.contains("1 feature(s)"));

    let after_push = storage::load_project(dir.path()).expect("load project after push");
    let pushed_view_box = after_push.component("view_box").expect("view_box exists");
    assert!(pushed_view_box.features[0].pushed);
    assert_eq!(
        pushed_view_box.features[0].external_id.as_deref(),
        Some("EXT-101")
    );

    let pull_out = execute_command(dir.path(), &adapters_path, Command::Pull { adapter: None })
        .expect("run pull");
    assert_eq!(pull_out.exit_code, 0);
    assert!(pull_out.stdout.contains("2 component(s) updated"));

    let after_pull = storage::load_project(dir.path()).expect("load project after pull");
    let synced_view_box = after_pull.component("view_box").expect("view_box exists");
    assert_eq!(synced_view_box.external_tasks.len(), 1);
    assert_eq!(
        synced_view_box.external_tasks[0].status,
        TaskStatus::Complete
    );
    let synced_stream = after_pull
        .component("data_stream")
        .expect("data_stream exists");
    assert_eq!(synced_stream.external_tasks.len(), 1);
    assert_eq!(synced_stream.external_tasks[0].status, TaskStatus::Complete);

    // 6. Audit --strict exits 0 now that all gaps are closed and features pushed
    let audit_after = execute_command(dir.path(), &adapters_path, Command::Audit { strict: true })
        .expect("run strict audit after sync");
    assert_eq!(audit_after.exit_code, 0);
    assert!(audit_after.stdout.contains("No audit findings."));
}

#[test]
fn e2e_tui_live_sync_and_complete_green_border_snapshot() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let adapters_path = dir.path().join("adapters.toml");

    storage::init_workspace(dir.path(), "mercury-tui", "Live TUI sync test")
        .expect("init workspace");

    let mut view_box = scaffold_component(
        "view_box",
        "View Box",
        ComponentKind::UiView,
        GridPos { col: 0, row: 0 },
    );
    view_box.features.truncate(1);
    storage::save_component(dir.path(), &view_box).expect("save view_box");

    let mut data_stream = scaffold_component(
        "data_stream",
        "Data Stream",
        ComponentKind::DataStream,
        GridPos { col: 1, row: 0 },
    );
    data_stream.features.truncate(1);
    storage::save_component(dir.path(), &data_stream).expect("save data_stream");

    let mut project = storage::load_project(dir.path()).expect("load project");
    project.config.active_adapters = vec!["mock_tracker".to_string()];
    project.config.connections.push(Connection {
        from: "view_box".to_string(),
        to: "data_stream".to_string(),
        kind: EdgeKind::DataFlow,
        label: None,
    });
    storage::save_project_config(dir.path(), &project.config).expect("save config");

    let pull_json = r#"{"components":[{"component_id":"view_box","tasks":[{"external_id":"EXT-301","title":"Define layout and container bounds","status":"complete"}]},{"component_id":"data_stream","tasks":[{"external_id":"EXT-302","title":"Define payload schema and framing","status":"complete"}]}]}"#;
    write_mock_adapters_toml(&adapters_path, pull_json);

    let reloaded = storage::load_project(dir.path()).expect("reload project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), reloaded, theme);

    let sync_msg = app.sync_pull(&adapters_path).expect("sync_pull succeeds");
    assert!(sync_msg.contains("2 component(s) updated"));
    assert_eq!(app.gap_count(), 0);

    let buffer = render_app(&mut app, 140, 40);
    let snapshot = buffer_to_string(&buffer);

    assert!(snapshot.contains("View Box"));
    assert!(snapshot.contains("Data Stream"));
    assert!(snapshot.contains("Tasks 1/1"));
    assert!(snapshot.contains('▶'));
    assert!(snapshot.contains("GAPS: 0"));

    let complete_rgb = app.theme.palette.status_complete;
    assert!(complete_rgb.g > complete_rgb.r);
    let expected_color = Color::Rgb(complete_rgb.r, complete_rgb.g, complete_rgb.b);
    let border_symbols = ["┌", "┐", "└", "┘", "─", "│", "╔", "╗", "╚", "╝", "═", "║"];

    let has_complete_green_border = (0..buffer.area.height).any(|y| {
        (0..buffer.area.width).any(|x| {
            let cell = &buffer[(x, y)];
            border_symbols.contains(&cell.symbol()) && cell.fg == expected_color
        })
    });
    assert!(
        has_complete_green_border,
        "expected at least one completed card border cell with status_complete color {expected_color:?}"
    );
}

#[test]
fn e2e_tui_viewport_panning_theme_switch_and_help_overlay() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let theme_dir = tempfile::tempdir().expect("create custom theme tempdir");

    storage::init_workspace(dir.path(), "gemini-ui", "Interactive TUI workflow test")
        .expect("init workspace");

    let origin = scaffold_component(
        "origin_hub",
        "Origin Hub",
        ComponentKind::ServiceCore,
        GridPos { col: 0, row: 0 },
    );
    storage::save_component(dir.path(), &origin).expect("save origin_hub");

    let remote = scaffold_component(
        "remote_relay",
        "Remote Relay",
        ComponentKind::DataStream,
        GridPos { col: 4, row: 2 },
    );
    storage::save_component(dir.path(), &remote).expect("save remote_relay");

    // Write custom `.ron` theme with distinct background/foreground RGB
    let custom_ron = DEFAULT_CRT_AMBER_RON
        .replace("\"crt_amber\"", "\"cobalt_test\"")
        .replace(
            "background: (r: 10, g: 10, b: 8)",
            "background: (r: 12, g: 24, b: 48)",
        )
        .replace(
            "foreground: (r: 255, g: 176, b: 0)",
            "foreground: (r: 180, g: 220, b: 255)",
        );
    fs::write(theme_dir.path().join("cobalt_test.ron"), custom_ron)
        .expect("write custom cobalt_test.ron");

    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);
    app.theme_dirs = vec![theme_dir.path().to_path_buf()];

    // 1. Select off-screen component at (col: 4, row: 2) and verify viewport panning
    let remote_idx = app
        .project
        .components
        .iter()
        .position(|c| c.id == "remote_relay")
        .expect("remote_relay index");
    app.selected_index = remote_idx;
    assert_eq!(app.viewport.scroll_col, 0);
    assert_eq!(app.viewport.scroll_row, 0);

    let panned_buffer = render_app(&mut app, 120, 35);
    let panned_text = buffer_to_string(&panned_buffer);
    assert!(
        app.viewport.scroll_col > 0 || app.viewport.scroll_row > 0,
        "expected viewport to pan to off-screen component"
    );
    assert!(panned_text.contains("Remote Relay"));

    // 2. Press 't' to open theme switcher, select `cobalt_test`, apply with Enter
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    let switcher_text = buffer_to_string(&render_app(&mut app, 120, 35));
    assert!(switcher_text.contains("Switch Theme"));
    assert!(switcher_text.contains("cobalt_test"));

    // Available themes are sorted: ["cobalt_test", "crt_amber"]; initial selection is "crt_amber" (1)
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.project.config.theme, "cobalt_test");
    assert_eq!(app.theme.name, "cobalt_test");

    // Verify theme persistence in `.rocket/project.toml`
    let persisted = storage::load_project(dir.path()).expect("reload persisted project");
    assert_eq!(persisted.config.theme, "cobalt_test");

    // Verify rendered buffer uses the new theme's background and foreground RGB colors
    let themed_buffer = render_app(&mut app, 120, 35);
    let expected_bg = Color::Rgb(12, 24, 48);
    let expected_fg = Color::Rgb(180, 220, 255);
    let has_new_bg = (0..themed_buffer.area.height)
        .any(|y| (0..themed_buffer.area.width).any(|x| themed_buffer[(x, y)].bg == expected_bg));
    let has_new_fg = (0..themed_buffer.area.height)
        .any(|y| (0..themed_buffer.area.width).any(|x| themed_buffer[(x, y)].fg == expected_fg));
    assert!(
        has_new_bg,
        "expected buffer to contain new theme background"
    );
    assert!(
        has_new_fg,
        "expected buffer to contain new theme foreground"
    );

    // 3. Press '?' to open Keyboard Help overlay and verify rendered keybinding docs
    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    let help_text = buffer_to_string(&render_app(&mut app, 120, 35));
    assert!(help_text.contains("Keyboard Help"));
    assert!(help_text.contains("Tab / Shift+Tab"));
    assert!(help_text.contains("Switch RON theme"));
    assert!(help_text.contains("Pull status from active adapters"));

    // Close help overlay
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.modal.is_none());
}

#[test]
fn e2e_multi_adapter_feature_push_cli_and_tui() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let adapters_path = dir.path().join("adapters.toml");
    let log_a = dir.path().join("tracker_a.log");
    let log_b = dir.path().join("tracker_b.log");

    storage::init_workspace(
        dir.path(),
        "gemini-multi",
        "Multi-adapter push verification for testuser@example.com",
    )
    .expect("init workspace");

    let mut cli_comp = scaffold_component(
        "cli_svc",
        "CLI Service",
        ComponentKind::ServiceCore,
        GridPos { col: 0, row: 0 },
    );
    cli_comp.features.truncate(1);
    storage::save_component(dir.path(), &cli_comp).expect("save cli_svc");

    let mut tui_comp = scaffold_component(
        "tui_view",
        "TUI View",
        ComponentKind::UiView,
        GridPos { col: 1, row: 0 },
    );
    tui_comp.features.truncate(1);
    storage::save_component(dir.path(), &tui_comp).expect("save tui_view");

    let mut project = storage::load_project(dir.path()).expect("load project");
    project.config.active_adapters = vec!["tracker_a".to_string(), "tracker_b".to_string()];
    storage::save_project_config(dir.path(), &project.config).expect("save project config");

    let adapters_file = AdaptersFile {
        adapters: vec![
            AdapterConfig {
                name: "tracker_a".to_string(),
                export_format: ExportFormat::Json,
                push_component_cmd: None,
                push_feature_cmd: Some(CommandSpec {
                    program: "sh".to_string(),
                    args: vec![
                        "-c".to_string(),
                        format!(
                            "cat >> '{}' && printf '{{\"external_id\":\"A-100\",\"message\":\"ok\"}}'",
                            log_a.display()
                        ),
                    ],
                }),
                pull_status_cmd: None,
            },
            AdapterConfig {
                name: "tracker_b".to_string(),
                export_format: ExportFormat::Json,
                push_component_cmd: None,
                push_feature_cmd: Some(CommandSpec {
                    program: "sh".to_string(),
                    args: vec![
                        "-c".to_string(),
                        format!(
                            "cat >> '{}' && printf '{{\"external_id\":\"B-200\",\"message\":\"ok\"}}'",
                            log_b.display()
                        ),
                    ],
                }),
                pull_status_cmd: None,
            },
        ],
    };
    fs::write(
        &adapters_path,
        toml::to_string_pretty(&adapters_file).expect("serialize adapters.toml"),
    )
    .expect("write adapters.toml");

    // 1. CLI push on `cli_svc`
    let cli_out = execute_command(
        dir.path(),
        &adapters_path,
        Command::Push {
            component: Some("cli_svc".to_string()),
            adapter: None,
        },
    )
    .expect("CLI multi-adapter push succeeds");
    assert_eq!(cli_out.exit_code, 0);
    assert!(cli_out.stdout.contains("2 feature(s) across 2 adapter(s)"));

    let log_a_after_cli = fs::read_to_string(&log_a).expect("read tracker_a log after CLI");
    let log_b_after_cli = fs::read_to_string(&log_b).expect("read tracker_b log after CLI");
    assert!(log_a_after_cli.contains("cli_svc_f1"));
    assert!(log_b_after_cli.contains("cli_svc_f1"));

    let after_cli = storage::load_project(dir.path()).expect("reload project after CLI push");
    let persisted_cli = after_cli.component("cli_svc").expect("cli_svc exists");
    assert!(persisted_cli.features[0].pushed);
    assert_eq!(
        persisted_cli.features[0].external_id.as_deref(),
        Some("A-100")
    );

    // 2. TUI push on `tui_view`
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), after_cli, theme);
    let tui_idx = app
        .project
        .components
        .iter()
        .position(|c| c.id == "tui_view")
        .expect("tui_view index");
    app.selected_index = tui_idx;

    let tui_summary = app
        .sync_push_selected(&adapters_path)
        .expect("TUI multi-adapter push succeeds");
    assert!(tui_summary.contains("2 feature(s) across 2 adapter(s)"));

    let log_a_after_tui = fs::read_to_string(&log_a).expect("read tracker_a log after TUI");
    let log_b_after_tui = fs::read_to_string(&log_b).expect("read tracker_b log after TUI");
    assert!(log_a_after_tui.contains("tui_view_f1"));
    assert!(log_b_after_tui.contains("tui_view_f1"));

    let after_tui = storage::load_project(dir.path()).expect("reload project after TUI push");
    let persisted_tui = after_tui.component("tui_view").expect("tui_view exists");
    assert!(persisted_tui.features[0].pushed);
    assert_eq!(
        persisted_tui.features[0].external_id.as_deref(),
        Some("A-100")
    );
}

#[test]
fn e2e_help_overlay_80x24_shows_close_footer_hint() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(dir.path(), "help-80x24", "80x24 help modal test")
        .expect("init workspace");
    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    let snapshot = buffer_to_string(&render_app(&mut app, 80, 24));
    assert!(
        snapshot.contains("[Esc/?/Enter/q] Close"),
        "expected help overlay footer hint to be visible at 80x24, got:\n{snapshot}"
    );
}
