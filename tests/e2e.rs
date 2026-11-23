use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use rocket_factory::adapter::{AdapterConfig, AdaptersFile, CommandSpec, ExportFormat};
use rocket_factory::archetype::scaffold_component;
use rocket_factory::cli::{execute_command, Command, ExportKind};
use rocket_factory::model::{
    ComponentKind, Connection, EdgeKind, FeatureItem, GridConfig, GridPos, TaskStatus,
};
use rocket_factory::storage;
use rocket_factory::theme::{default_theme, DEFAULT_CRT_AMBER_RON};
use rocket_factory::tui::app::App;
use rocket_factory::tui::canvas::ZoomLevel;
use rocket_factory::tui::ui::{column_layout, render};

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
    view_box.features.push(FeatureItem {
        id: "view_box_f1".to_string(),
        title: "Define layout and container bounds".to_string(),
        pushed: false,
        external_id: None,
    });
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
    view_box.features.push(FeatureItem {
        id: "view_box_f1".to_string(),
        title: "Define layout and container bounds".to_string(),
        pushed: false,
        external_id: None,
    });
    storage::save_component(dir.path(), &view_box).expect("save view_box");

    let mut data_stream = scaffold_component(
        "data_stream",
        "Data Stream",
        ComponentKind::DataStream,
        GridPos { col: 1, row: 0 },
    );
    data_stream.features.push(FeatureItem {
        id: "data_stream_f1".to_string(),
        title: "Define payload schema and framing".to_string(),
        pushed: false,
        external_id: None,
    });
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
    cli_comp.features.push(FeatureItem {
        id: "cli_svc_f1".to_string(),
        title: "Define domain API and error contracts".to_string(),
        pushed: false,
        external_id: None,
    });
    storage::save_component(dir.path(), &cli_comp).expect("save cli_svc");

    let mut tui_comp = scaffold_component(
        "tui_view",
        "TUI View",
        ComponentKind::UiView,
        GridPos { col: 1, row: 0 },
    );
    tui_comp.features.push(FeatureItem {
        id: "tui_view_f1".to_string(),
        title: "Define layout and container bounds".to_string(),
        pushed: false,
        external_id: None,
    });
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

#[test]
fn e2e_tui_delete_connection_and_component_persists_to_disk() {
    use rocket_factory::audit::{audit_project, FindingKind};

    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "delete-e2e",
        "Delete component and connection E2E for testuser@example.com",
    )
    .expect("init workspace");

    let alpha = scaffold_component(
        "alpha_core",
        "Alpha Core",
        ComponentKind::ServiceCore,
        GridPos { col: 0, row: 0 },
    );
    let beta = scaffold_component(
        "beta_stream",
        "Beta Stream",
        ComponentKind::DataStream,
        GridPos { col: 1, row: 0 },
    );
    let gamma = scaffold_component(
        "gamma_view",
        "Gamma View",
        ComponentKind::UiView,
        GridPos { col: 2, row: 0 },
    );
    storage::save_component(dir.path(), &alpha).expect("save alpha_core");
    storage::save_component(dir.path(), &beta).expect("save beta_stream");
    storage::save_component(dir.path(), &gamma).expect("save gamma_view");

    let mut project = storage::load_project(dir.path()).expect("load project");
    project.config.connections = vec![
        Connection {
            from: "alpha_core".to_string(),
            to: "beta_stream".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        },
        Connection {
            from: "beta_stream".to_string(),
            to: "gamma_view".to_string(),
            kind: EdgeKind::DataFlow,
            label: Some("events".to_string()),
        },
        Connection {
            from: "gamma_view".to_string(),
            to: "alpha_core".to_string(),
            kind: EdgeKind::Renders,
            label: None,
        },
    ];
    storage::save_project_config(dir.path(), &project.config).expect("save project config");

    let loaded_init = storage::load_project(dir.path()).expect("reload project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), loaded_init, theme);

    // 1. Press 'D' + 'y' on alpha_core to delete outgoing connection alpha_core -> beta_stream
    app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));
    let confirm_conn_snap = buffer_to_string(&render_app(&mut app, 120, 35));
    assert!(confirm_conn_snap.contains("Delete Connection"));
    assert!(confirm_conn_snap.contains("alpha_core -> beta_stream"));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let after_conn_delete = storage::load_project(dir.path()).expect("reload after conn delete");
    assert_eq!(after_conn_delete.config.connections.len(), 2);
    assert!(!after_conn_delete
        .config
        .connections
        .iter()
        .any(|c| c.from == "alpha_core" && c.to == "beta_stream"));
    assert!(!audit_project(&after_conn_delete)
        .iter()
        .any(|f| f.kind == FindingKind::DanglingConnection));

    // 2. Press 'd' + 'n' on alpha_core to cancel component deletion
    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    let confirm_comp_snap = buffer_to_string(&render_app(&mut app, 120, 35));
    assert!(confirm_comp_snap.contains("Delete Component"));
    assert!(confirm_comp_snap.contains("Alpha Core"));
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

    let after_cancel = storage::load_project(dir.path()).expect("reload after cancel");
    assert!(after_cancel.component("alpha_core").is_some());
    assert_eq!(after_cancel.components.len(), 3);
    assert_eq!(after_cancel.config.connections.len(), 2);
    assert!(!audit_project(&after_cancel)
        .iter()
        .any(|f| f.kind == FindingKind::DanglingConnection));

    // 3. Select beta_stream (has incident edge beta_stream -> gamma_view) and press 'd' + 'y'
    let beta_idx = app
        .project
        .components
        .iter()
        .position(|c| c.id == "beta_stream")
        .expect("beta_stream index");
    app.selected_index = beta_idx;
    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let after_comp_delete = storage::load_project(dir.path()).expect("reload after comp delete");
    assert!(after_comp_delete.component("beta_stream").is_none());
    assert_eq!(after_comp_delete.components.len(), 2);
    assert_eq!(after_comp_delete.config.connections.len(), 1);
    assert_eq!(after_comp_delete.config.connections[0].from, "gamma_view");
    assert_eq!(after_comp_delete.config.connections[0].to, "alpha_core");
    assert!(!audit_project(&after_comp_delete)
        .iter()
        .any(|f| f.kind == FindingKind::DanglingConnection));
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
}

fn select_component_by_id(app: &mut App, id: &str) {
    let visible_len = app.visible_component_indices().len();
    assert!(
        visible_len > 0,
        "expected visible component '{id}', but component list is empty"
    );
    let target_idx = app
        .visible_component_indices()
        .iter()
        .position(|&idx| app.project.components.get(idx).is_some_and(|c| c.id == id))
        .unwrap_or_else(|| panic!("expected visible component '{id}'"));

    let nav_key = if target_idx >= app.selected_index {
        KeyCode::Down
    } else {
        KeyCode::Up
    };

    for _ in 0..visible_len {
        if app.selected_component().is_some_and(|c| c.id == id) {
            return;
        }
        app.handle_key(key(nav_key));
    }

    assert_eq!(
        app.selected_component().map(|c| c.id.as_str()),
        Some(id),
        "failed to navigate to visible component '{id}' via key events"
    );
}

fn rect_to_string(buffer: &Buffer, rect: Rect) -> String {
    let mut text = String::new();
    let max_x = rect.x.saturating_add(rect.width).min(buffer.area.width);
    let max_y = rect.y.saturating_add(rect.height).min(buffer.area.height);
    for y in rect.y..max_y {
        for x in rect.x..max_x {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn e2e_sub_component_creation_expand_collapse_and_collision_shifting() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "apollo-subcomp",
        "Sub-component hierarchy E2E for testuser@example.com",
    )
    .expect("init workspace");

    let api_gateway = scaffold_component(
        "api_gateway",
        "API Gateway",
        ComponentKind::Interface,
        GridPos { col: 0, row: 0 },
    );
    let auth_svc = scaffold_component(
        "auth_svc",
        "Auth Svc",
        ComponentKind::Logic,
        GridPos { col: 1, row: 0 },
    );
    storage::save_component(dir.path(), &api_gateway).expect("save api_gateway");
    storage::save_component(dir.path(), &auth_svc).expect("save auth_svc");

    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    // 1. Create sub-component `rate_limiter` under `api_gateway` using `a`
    select_component_by_id(&mut app, "api_gateway");
    app.handle_key(key(KeyCode::Char('a')));
    type_text(&mut app, "rate_limiter");
    app.handle_key(key(KeyCode::Tab));
    type_text(&mut app, "Rate Limiter");
    app.handle_key(key(KeyCode::Enter));
    assert!(app.modal.is_none());

    let after_create = storage::load_project(dir.path()).expect("reload after sub-comp create");
    let rate_limiter = after_create
        .component("rate_limiter")
        .expect("rate_limiter persisted on disk");
    let persisted_gateway = after_create
        .component("api_gateway")
        .expect("api_gateway persisted");
    let persisted_auth = after_create
        .component("auth_svc")
        .expect("auth_svc persisted");

    assert_eq!(rate_limiter.parent_id.as_deref(), Some("api_gateway"));
    assert_ne!(rate_limiter.position, persisted_gateway.position);
    assert_ne!(rate_limiter.position, persisted_auth.position);
    let rate_pos = rate_limiter.position;

    let canvas_rect = column_layout(Rect::new(0, 3, 140, 36))[1];
    let expanded_buf = render_app(&mut app, 140, 40);
    let expanded_canvas = rect_to_string(&expanded_buf, canvas_rect);
    assert!(
        expanded_canvas.contains("Rate Li"),
        "expected rate_limiter visible on canvas:\n{expanded_canvas}"
    );
    assert!(
        expanded_canvas.contains("[-1]"),
        "expected [-1] badge on api_gateway:\n{expanded_canvas}"
    );

    // 2. Collapse `api_gateway` via Space
    select_component_by_id(&mut app, "api_gateway");
    app.handle_key(key(KeyCode::Char(' ')));

    let visible_ids_collapsed: Vec<&str> = app
        .visible_component_indices()
        .into_iter()
        .filter_map(|idx| app.project.components.get(idx).map(|c| c.id.as_str()))
        .collect();
    assert!(!visible_ids_collapsed.contains(&"rate_limiter"));

    let collapsed_buf = render_app(&mut app, 140, 40);
    let collapsed_canvas = rect_to_string(&collapsed_buf, canvas_rect);
    assert!(
        collapsed_canvas.contains("[+1]"),
        "expected [+1] badge on collapsed api_gateway:\n{collapsed_canvas}"
    );
    assert!(!collapsed_canvas.contains("Rate Li"));

    // 3. Seed collision by moving `auth_svc` onto `rate_limiter`'s hidden GridPos on disk and reloading
    let mut colliding_auth = persisted_auth.clone();
    colliding_auth.position = rate_pos;
    storage::save_component(dir.path(), &colliding_auth).expect("save colliding auth_svc");

    let reloaded_project = storage::load_project(dir.path()).expect("reload colliding project");
    let theme2 = default_theme().expect("load default theme");
    let mut app2 = App::new(dir.path().to_path_buf(), reloaded_project, theme2);

    // Expand `api_gateway` via Space and verify collision shifting
    select_component_by_id(&mut app2, "api_gateway");
    app2.handle_key(key(KeyCode::Char(' ')));

    let visible_indices_after = app2.visible_component_indices();
    let visible_ids_after: Vec<&str> = visible_indices_after
        .iter()
        .filter_map(|&idx| app2.project.components.get(idx).map(|c| c.id.as_str()))
        .collect();
    assert!(visible_ids_after.contains(&"rate_limiter"));
    assert!(visible_ids_after.contains(&"auth_svc"));

    let visible_positions: HashSet<GridPos> = visible_indices_after
        .iter()
        .filter_map(|&idx| app2.project.components.get(idx).map(|c| c.position))
        .collect();
    assert_eq!(
        visible_positions.len(),
        visible_indices_after.len(),
        "no two visible components should share a GridPos after expand"
    );

    let after_expand = storage::load_project(dir.path()).expect("reload after expand shift");
    let final_rate = after_expand
        .component("rate_limiter")
        .expect("rate_limiter exists");
    let final_auth = after_expand.component("auth_svc").expect("auth_svc exists");
    let final_gw = after_expand
        .component("api_gateway")
        .expect("api_gateway exists");

    assert_eq!(final_rate.parent_id.as_deref(), Some("api_gateway"));
    assert_eq!(final_rate.position, rate_pos);
    assert_ne!(final_auth.position, final_rate.position);
    assert_ne!(final_auth.position, final_gw.position);
}

#[test]
fn e2e_edge_contains_ingestion_mutable_archetype_and_connect_semantics() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "gemini-semantics",
        "Edge contains & archetype mutation E2E for testuser@example.com",
    )
    .expect("init workspace");

    let custom_desc = "Primary web client for example.com".to_string();
    let custom_body = "## Custom Notes\n\nKeep this blueprint body intact.\n".to_string();

    let mut frontend = scaffold_component(
        "frontend",
        "Frontend",
        ComponentKind::Concept,
        GridPos { col: 0, row: 0 },
    );
    frontend.description = custom_desc.clone();
    frontend.markdown_body = custom_body.clone();
    let backend = scaffold_component(
        "backend",
        "Backend",
        ComponentKind::Concept,
        GridPos { col: 1, row: 0 },
    );
    let widget_lib = scaffold_component(
        "widget_lib",
        "Widget Lib",
        ComponentKind::Concept,
        GridPos { col: 2, row: 0 },
    );
    storage::save_component(dir.path(), &frontend).expect("save frontend");
    storage::save_component(dir.path(), &backend).expect("save backend");
    storage::save_component(dir.path(), &widget_lib).expect("save widget_lib");

    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    // 1. Drive `c` on `frontend`, navigate to `backend`, confirm with Enter
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('c')));
    assert!(
        app.status_message.contains("depends on <target>"),
        "unexpected connect prompt: {}",
        app.status_message
    );

    let mut steps = 0;
    while app.selected_component().map(|c| c.id.as_str()) != Some("backend") {
        app.handle_key(key(KeyCode::Down));
        steps += 1;
        assert!(steps <= 10, "failed to navigate to backend");
    }
    app.handle_key(key(KeyCode::Enter));
    assert!(
        app.status_message.contains("frontend depends on backend"),
        "unexpected connect confirmation: {}",
        app.status_message
    );

    let after_connect = storage::load_project(dir.path()).expect("reload after connect");
    assert!(after_connect
        .config
        .connections
        .iter()
        .any(|c| { c.from == "frontend" && c.to == "backend" && c.kind == EdgeKind::DependsOn }));

    // 2. Reselect `frontend` and delete `frontend -> backend` via `D` + `y`
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('D')));
    app.handle_key(key(KeyCode::Char('y')));
    let after_del_conn = storage::load_project(dir.path()).expect("reload after delete conn");
    assert!(after_del_conn.config.connections.is_empty());

    // 3. Drive `e` on `frontend`: change archetype to Interface and add sub-component
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('e')));
    // Field 0 (Name) -> Tab -> Field 1 (Archetype: Concept -> Right -> Interface)
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Right));
    // Field 1 -> Tab x5 -> Field 6 (Add Sub-component)
    for _ in 0..5 {
        app.handle_key(key(KeyCode::Tab));
    }
    type_text(&mut app, "Nav Header");
    app.handle_key(key(KeyCode::Enter));
    assert!(app.modal.is_none());

    let after_edit = storage::load_project(dir.path()).expect("reload after edit component");
    let edited_frontend = after_edit
        .component("frontend")
        .expect("frontend still exists");
    assert_eq!(edited_frontend.kind, ComponentKind::Interface);
    assert_eq!(edited_frontend.description, custom_desc);
    assert_eq!(edited_frontend.markdown_body, custom_body);
    let nav_header = after_edit
        .component("nav_header")
        .expect("new sub-component nav_header persisted");
    assert_eq!(nav_header.parent_id.as_deref(), Some("frontend"));

    // 4. Reselect `frontend`, connect `frontend -> widget_lib`, then edit edge via `E` to Contains
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('c')));
    steps = 0;
    while app.selected_component().map(|c| c.id.as_str()) != Some("widget_lib") {
        app.handle_key(key(KeyCode::Down));
        steps += 1;
        assert!(steps <= 10, "failed to navigate to widget_lib");
    }
    app.handle_key(key(KeyCode::Enter));

    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('E')));
    // EDGE_KINDS: DependsOn(0) -> DataFlow(1) -> Renders(2) -> Invokes(3) -> Contains(4)
    for _ in 0..4 {
        app.handle_key(key(KeyCode::Right));
    }
    app.handle_key(key(KeyCode::Enter));
    assert!(app.modal.is_none());

    let after_contains = storage::load_project(dir.path()).expect("reload after Contains edge");
    assert_eq!(
        after_contains
            .component("widget_lib")
            .expect("widget_lib exists")
            .parent_id
            .as_deref(),
        Some("frontend")
    );

    // 5. Drive `E` again on `frontend` to change Contains back to DependsOn and verify detachment
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('E')));
    for _ in 0..4 {
        app.handle_key(key(KeyCode::Left));
    }
    app.handle_key(key(KeyCode::Enter));
    assert!(app.modal.is_none());

    let after_detach = storage::load_project(dir.path()).expect("reload after DependsOn edge");
    assert_eq!(
        after_detach
            .component("widget_lib")
            .expect("widget_lib exists")
            .parent_id,
        None
    );
}

#[test]
fn e2e_canvas_zoom_levels_and_compact_4x4_capacity() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "zoom-4x4",
        "Zoom levels and 4x4 compact capacity E2E for testuser@example.com",
    )
    .expect("init workspace");

    let mut short_names = Vec::new();
    for row in 0..4u16 {
        for col in 0..4u16 {
            let id = format!("c{row}{col}");
            let name = format!("C{row}{col}");
            let comp = scaffold_component(&id, &name, ComponentKind::Concept, GridPos { col, row });
            storage::save_component(dir.path(), &comp).expect("save grid component");
            short_names.push(name);
        }
    }

    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    assert_eq!(app.viewport.zoom, ZoomLevel::Compact);

    let canvas_rect = column_layout(Rect::new(0, 3, 140, 36))[1];
    let compact_buf = render_app(&mut app, 140, 40);
    let compact_canvas = rect_to_string(&compact_buf, canvas_rect);
    assert!(
        compact_canvas.contains("[Compact]"),
        "expected [Compact] in canvas title:\n{compact_canvas}"
    );
    for name in &short_names {
        assert!(
            compact_canvas.contains(name.as_str()),
            "expected card '{name}' inside 4x4 Compact canvas rect:\n{compact_canvas}"
        );
    }

    // Zoom in via `+` -> Normal
    app.handle_key(key(KeyCode::Char('+')));
    assert_eq!(app.viewport.zoom, ZoomLevel::Normal);
    let normal_canvas = rect_to_string(&render_app(&mut app, 140, 40), canvas_rect);
    assert!(
        normal_canvas.contains("[Normal]"),
        "expected [Normal] in canvas title:\n{normal_canvas}"
    );

    // Zoom in via `=` -> Expanded
    app.handle_key(key(KeyCode::Char('=')));
    assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);
    let expanded_canvas = rect_to_string(&render_app(&mut app, 140, 40), canvas_rect);
    assert!(
        expanded_canvas.contains("[Expanded]"),
        "expected [Expanded] in canvas title:\n{expanded_canvas}"
    );

    // Select a card at col 3 (row 0, col 3 -> c03 / C03) via key navigation and render at 140x40
    select_component_by_id(&mut app, "c03");
    let _ = render_app(&mut app, 140, 40);
    assert!(
        app.viewport.scroll_col > 0,
        "expected viewport.scroll_col > 0 after selecting col-3 card in Expanded zoom, got {}",
        app.viewport.scroll_col
    );

    // Zoom out twice via `-` -> Compact
    app.handle_key(key(KeyCode::Char('-')));
    app.handle_key(key(KeyCode::Char('-')));
    assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
    let back_compact_buf = render_app(&mut app, 140, 40);
    let back_compact_canvas = rect_to_string(&back_compact_buf, canvas_rect);
    assert!(
        back_compact_canvas.contains("[Compact]"),
        "expected [Compact] after zooming back out:\n{back_compact_canvas}"
    );
    assert_eq!(
        app.viewport.scroll_col, 0,
        "expected zoom-out to Compact to pull viewport.scroll_col back to 0"
    );
    for name in ["C00", "C01", "C02", "C03"] {
        assert!(
            back_compact_canvas.contains(name),
            "expected row-0 card '{name}' visible in Compact canvas rect after zoom-out:\n{back_compact_canvas}"
        );
    }

    // Inspect bottom footer row (y = 39) for underlined hotkey cells and absence of legacy brackets
    let footer_row = rect_to_string(&back_compact_buf, Rect::new(0, 39, 140, 1));
    assert!(
        !footer_row.contains("[v] Edit MD"),
        "legacy '[v] Edit MD' bracketed hint should not appear in footer: {footer_row}"
    );
    let underlined_cells = (0..140u16)
        .filter(|&x| {
            back_compact_buf[(x, 39)]
                .modifier
                .contains(Modifier::UNDERLINED)
        })
        .count();
    assert!(
        underlined_cells > 0,
        "expected underlined mnemonic hotkey cells in bottom footer row (y = 39)"
    );
}

#[test]
fn e2e_top_center_default_placement_and_grid_expansion_persists() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "top-center-e2e",
        "Top-center default placement E2E for testuser@example.com",
    )
    .expect("init workspace");

    let mut cfg = storage::load_project(dir.path())
        .expect("load initial project")
        .config;
    cfg.grid = GridConfig { cols: 1, rows: 1 };
    storage::save_project_config(dir.path(), &cfg).expect("save 1x1 grid config");

    let project = storage::load_project(dir.path()).expect("load 1x1 project");
    assert_eq!(project.config.grid, GridConfig { cols: 1, rows: 1 });
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    // Render once at 140x40 so last_canvas_capacity is populated
    let _ = render_app(&mut app, 140, 40);

    // Drive `n` to create top-level component `core_hub` beyond the saved 1x1 grid bounds
    app.handle_key(key(KeyCode::Char('n')));
    type_text(&mut app, "core_hub");
    app.handle_key(key(KeyCode::Tab));
    type_text(&mut app, "Core Hub");
    app.handle_key(key(KeyCode::Enter));
    assert!(app.modal.is_none());

    let core_hub = app
        .project
        .component("core_hub")
        .expect("core_hub created in app state");
    assert_eq!(core_hub.position.row, 0);
    assert!(
        core_hub.position.col > 0,
        "expected top-center placement with col > 0, got {:?}",
        core_hub.position
    );
    assert!(
        app.project.config.grid.cols > 1 || app.project.config.grid.rows > 1,
        "expected saved 1x1 grid to expand after creating core_hub at {:?}, got {:?}",
        core_hub.position,
        app.project.config.grid
    );
    assert!(
        app.project.config.grid.cols > core_hub.position.col,
        "expected grid.cols ({}) > core_hub.position.col ({})",
        app.project.config.grid.cols,
        core_hub.position.col
    );

    let reloaded = storage::load_project(dir.path()).expect("reload project from disk");
    let persisted = reloaded
        .component("core_hub")
        .expect("core_hub persisted on disk");
    assert_eq!(persisted.position, core_hub.position);
    assert_eq!(reloaded.config.grid, app.project.config.grid);
    assert!(
        reloaded.config.grid.cols > 1 || reloaded.config.grid.rows > 1,
        "expected expanded grid dimensions persisted on disk, got {:?}",
        reloaded.config.grid
    );
}

#[test]
fn e2e_vertical_scroll_and_minimum_4x4_compact_canvas() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "zoom-vert-4x4",
        "Vertical zoom scroll and 114x26 4x4 Compact capacity E2E for testuser@example.com",
    )
    .expect("init workspace");

    let mut short_names = Vec::new();
    for row in 0..4u16 {
        for col in 0..4u16 {
            let id = format!("c{row}{col}");
            let name = format!("C{row}{col}");
            let comp = scaffold_component(&id, &name, ComponentKind::Concept, GridPos { col, row });
            storage::save_component(dir.path(), &comp).expect("save grid component");
            short_names.push(name);
        }
    }

    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    assert_eq!(app.viewport.zoom, ZoomLevel::Compact);

    let wide_canvas_rect = column_layout(Rect::new(0, 3, 140, 36))[1];
    assert!(
        wide_canvas_rect.width * 100 / 140 >= 60,
        "expected canvas width >= 60% of 140 cols, got {}",
        wide_canvas_rect.width
    );

    // Render at minimum 114x26 terminal in Compact zoom and verify all 16 cards fit
    let min_canvas_rect = column_layout(Rect::new(0, 3, 114, 22))[1];
    let min_compact_buf = render_app(&mut app, 114, 26);
    let min_compact_canvas = rect_to_string(&min_compact_buf, min_canvas_rect);
    for name in &short_names {
        assert!(
            min_compact_canvas.contains(name.as_str()),
            "expected card '{name}' inside 114x26 Compact canvas rect:\n{min_compact_canvas}"
        );
    }

    // Zoom in via `+` and `=` -> Expanded
    app.handle_key(key(KeyCode::Char('+')));
    assert_eq!(app.viewport.zoom, ZoomLevel::Normal);
    app.handle_key(key(KeyCode::Char('=')));
    assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);

    // Select row-3 card `c30` via real key navigation and render at 140x40
    select_component_by_id(&mut app, "c30");
    let _ = render_app(&mut app, 140, 40);
    assert!(
        app.viewport.scroll_row > 0,
        "expected viewport.scroll_row > 0 after selecting row-3 card in Expanded zoom, got {}",
        app.viewport.scroll_row
    );

    // Zoom out twice via `-` -> Compact and verify vertical scroll pullback
    app.handle_key(key(KeyCode::Char('-')));
    app.handle_key(key(KeyCode::Char('-')));
    assert_eq!(app.viewport.zoom, ZoomLevel::Compact);

    let back_compact_buf = render_app(&mut app, 140, 40);
    let back_compact_canvas = rect_to_string(&back_compact_buf, wide_canvas_rect);
    assert_eq!(
        app.viewport.scroll_row, 0,
        "expected zoom-out to Compact to pull viewport.scroll_row back to 0"
    );
    for name in ["C00", "C10", "C20", "C30"] {
        assert!(
            back_compact_canvas.contains(name),
            "expected column-0 card '{name}' visible in Compact canvas rect after zoom-out:\n{back_compact_canvas}"
        );
    }
}

#[test]
fn e2e_help_modal_connect_direction_and_edit_edge_preview() {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    storage::init_workspace(
        dir.path(),
        "rf-e2e",
        "Help modal, connect direction, and edit edge preview E2E for testuser@example.com",
    )
    .expect("init workspace");

    let frontend = scaffold_component(
        "frontend",
        "Frontend",
        ComponentKind::Interface,
        GridPos { col: 0, row: 0 },
    );
    let backend = scaffold_component(
        "backend",
        "Backend",
        ComponentKind::Logic,
        GridPos { col: 1, row: 0 },
    );
    storage::save_component(dir.path(), &frontend).expect("save frontend");
    storage::save_component(dir.path(), &backend).expect("save backend");

    let project = storage::load_project(dir.path()).expect("load project");
    let theme = default_theme().expect("load default theme");
    let mut app = App::new(dir.path().to_path_buf(), project, theme);

    // Press `?` to open Keyboard Help modal and verify at 80x24 and 140x40
    app.handle_key(key(KeyCode::Char('?')));
    for (width, height) in [(80, 24), (140, 40)] {
        let rendered = buffer_to_string(&render_app(&mut app, width, height));
        for expected in [
            "Keyboard Help",
            "Connect (A depends on B)",
            "Zoom in",
            "Zoom out",
            "[Esc/?/Enter/q] Close",
        ] {
            assert!(
                rendered.contains(expected),
                "expected '{expected}' in Help modal at {width}x{height}:\n{rendered}"
            );
        }
    }

    // Press `?` again to close Help modal
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.modal.is_none());

    // Select `frontend`, press `c`, and verify rendered header prompt direction
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('c')));
    let connect_buf = render_app(&mut app, 140, 40);
    let header_text = rect_to_string(&connect_buf, Rect::new(0, 0, 140, 3));
    assert!(
        header_text.contains("Frontend depends on <target>"),
        "expected connect header prompt to clarify direction:\n{header_text}"
    );

    // Navigate via real key events to `backend` and press Enter to confirm connection
    select_component_by_id(&mut app, "backend");
    app.handle_key(key(KeyCode::Enter));

    // Reselect `frontend`, press `E` to open Edit Connection modal, and verify edge preview
    select_component_by_id(&mut app, "frontend");
    app.handle_key(key(KeyCode::Char('E')));
    let edit_edge_rendered = buffer_to_string(&render_app(&mut app, 140, 40));
    assert!(
        edit_edge_rendered.contains("Edit Connection"),
        "expected 'Edit Connection' modal title:\n{edit_edge_rendered}"
    );
    assert!(
        edit_edge_rendered.contains("Edge: frontend depends on backend"),
        "expected 'Edge: frontend depends on backend' in Edit Connection modal:\n{edit_edge_rendered}"
    );

    // Press Esc to close Edit Connection modal
    app.handle_key(key(KeyCode::Esc));
    assert!(app.modal.is_none());
}

#[test]
fn e2e_readme_documents_release_build_and_tui_features() {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let readme = fs::read_to_string(&readme_path).expect("read README.md");

    assert!(
        readme.contains("### Build and install from a local clone"),
        "README.md missing local clone build heading"
    );

    let install_pos = readme
        .find("cargo install --path . --locked")
        .expect("README.md missing cargo install --path . --locked");
    let build_pos = readme
        .find("cargo build --release --locked")
        .expect("README.md missing cargo build --release --locked");
    assert!(
        install_pos < build_pos,
        "expected cargo install block before cargo build --release block"
    );
    assert!(
        readme[install_pos..build_pos].contains("```"),
        "expected cargo install and cargo build --release in separate fenced code blocks"
    );

    for binding in [
        "| `+` / `=` |",
        "| `-` |",
        "| `c` |",
        "| `a` |",
        "| `E` |",
        "<A> depends on <target>",
    ] {
        assert!(
            readme.contains(binding),
            "README.md missing TUI keyboard documentation '{binding}'"
        );
    }

    for doc_token in [
        "`concept`",
        "`interface`",
        "`logic`",
        "`data`",
        "`integration`",
        "`custom`",
        "`parent_id`",
    ] {
        assert!(
            readme.contains(doc_token),
            "README.md missing archetype/sub-component documentation '{doc_token}'"
        );
    }
}
