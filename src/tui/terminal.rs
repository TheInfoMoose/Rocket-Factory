use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::error::{RbError, RbResult};
use crate::storage::{save_component, validate_component_id, COMPONENTS_DIR, ROCKET_DIR};
use crate::tui::app::{App, SyncRequest};
use crate::tui::ui;

pub type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

fn terminal_io_err(source: io::Error) -> RbError {
    RbError::Io {
        path: PathBuf::from("<terminal>"),
        source,
    }
}

pub fn default_theme_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.join(".rocket/themes")];
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            dirs.push(PathBuf::from(home).join(".config/rocket-bench/themes"));
        }
    }
    dirs
}

pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

pub fn enter_terminal() -> RbResult<TuiTerminal> {
    enable_raw_mode().map_err(terminal_io_err)?;
    if let Err(err) = execute!(io::stdout(), EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(terminal_io_err(err));
    }
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend).map_err(|err| {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        terminal_io_err(err)
    })
}

pub fn restore_terminal(terminal: &mut TuiTerminal) -> RbResult<()> {
    disable_raw_mode().map_err(terminal_io_err)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(terminal_io_err)?;
    terminal.show_cursor().map_err(terminal_io_err)?;
    Ok(())
}

fn parse_editor_command(editor_value: Option<&str>) -> (String, Vec<String>) {
    if let Some(raw) = editor_value {
        let mut parts = raw.split_ascii_whitespace();
        if let Some(program) = parts.next() {
            let args = parts.map(str::to_string).collect();
            return (program.to_string(), args);
        }
    }
    ("vi".to_string(), Vec::new())
}

pub fn suspend_for_editor(terminal: &mut TuiTerminal, file_path: &Path) -> RbResult<()> {
    restore_terminal(terminal)?;

    let editor_env = std::env::var("EDITOR").ok();
    let (program, args) = parse_editor_command(editor_env.as_deref());

    let status_res = Command::new(&program).args(&args).arg(file_path).status();

    enable_raw_mode().map_err(terminal_io_err)?;
    execute!(terminal.backend_mut(), EnterAlternateScreen).map_err(terminal_io_err)?;
    terminal.clear().map_err(terminal_io_err)?;

    let status = status_res.map_err(|source| RbError::Io {
        path: file_path.to_path_buf(),
        source,
    })?;

    if !status.success() {
        return Err(RbError::Io {
            path: file_path.to_path_buf(),
            source: io::Error::other(format!("editor '{program}' exited with status {status}")),
        });
    }

    Ok(())
}

fn process_pending_editor_with<F>(app: &mut App, mut run_editor: F)
where
    F: FnMut(&Path) -> RbResult<()>,
{
    let Some(comp_id) = app.pending_editor_component.take() else {
        return;
    };

    if let Err(err) = validate_component_id(&comp_id) {
        app.status_message = format!("Invalid component ID '{comp_id}': {err}");
        return;
    }

    let Some(comp) = app.project.component(&comp_id) else {
        app.status_message = format!("Component '{comp_id}' not found");
        return;
    };

    let components_dir = app.root.join(ROCKET_DIR).join(COMPONENTS_DIR);
    let md_path = components_dir.join(format!("{comp_id}.md"));

    if md_path.symlink_metadata().is_err() {
        if let Err(err) = save_component(&app.root, comp) {
            app.status_message = format!("Failed to save {comp_id} before editing: {err}");
            return;
        }
    }

    let canon_dir = match std::fs::canonicalize(&components_dir) {
        Ok(path) => path,
        Err(err) => {
            app.status_message =
                format!("Failed to resolve components directory for {comp_id}: {err}");
            return;
        }
    };

    let canon_md = match std::fs::canonicalize(&md_path) {
        Ok(path) => path,
        Err(err) => {
            app.status_message = format!("Failed to resolve Markdown path for {comp_id}: {err}");
            return;
        }
    };

    if !canon_md.starts_with(&canon_dir) {
        app.status_message = format!("Refusing to edit {comp_id}: path escapes components dir");
        return;
    }

    match run_editor(&canon_md) {
        Ok(()) => {
            if let Err(err) = app.reload_component_markdown(&comp_id) {
                app.status_message = format!("Failed to reload Markdown for {comp_id}: {err}");
            }
        }
        Err(err) => {
            app.status_message = format!("Editor error for {comp_id}: {err}");
        }
    }
}

fn process_pending_sync(app: &mut App, adapters_path: &Path) {
    let Some(request) = app.pending_sync.take() else {
        return;
    };

    let result = match request {
        SyncRequest::Pull => app.sync_pull(adapters_path),
        SyncRequest::PushSelected => app.sync_push_selected(adapters_path),
    };

    if let Err(err) = result {
        app.status_message = format!("Adapter sync failed: {err}");
    }
}

fn build_tui_app(root: &Path, theme_dirs: &[PathBuf]) -> RbResult<App> {
    let project = crate::storage::load_project(root)?;
    let theme = crate::theme::resolve_theme(&project.config.theme, theme_dirs)?;
    let mut app = App::new(root.to_path_buf(), project, theme);
    app.theme_dirs = theme_dirs.to_vec();
    Ok(app)
}

pub fn run_tui(
    root: &Path,
    theme_dirs: &[PathBuf],
    adapters_path: &Path,
    poll_interval: Option<Duration>,
) -> RbResult<()> {
    let mut app = build_tui_app(root, theme_dirs)?;

    if !app.project.config.active_adapters.is_empty() {
        if let Err(err) = app.sync_pull(adapters_path) {
            app.status_message = format!("Initial adapter sync failed: {err}");
        }
    }

    install_panic_hook();
    let mut terminal = enter_terminal()?;
    let loop_result = run_event_loop(&mut terminal, &mut app, adapters_path, poll_interval);
    let restore_result = restore_terminal(&mut terminal);

    loop_result.and(restore_result)
}

fn run_event_loop(
    terminal: &mut TuiTerminal,
    app: &mut App,
    adapters_path: &Path,
    poll_interval: Option<Duration>,
) -> RbResult<()> {
    let mut last_poll = Instant::now();

    loop {
        terminal
            .draw(|f| ui::render(f, app))
            .map_err(terminal_io_err)?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100)).map_err(terminal_io_err)? {
            if let Event::Key(key) = event::read().map_err(terminal_io_err)? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                    process_pending_editor_with(app, |path| suspend_for_editor(terminal, path));
                    process_pending_sync(app, adapters_path);
                }
            }
        }

        if app.should_quit {
            break;
        }

        if let Some(interval) = poll_interval {
            if !app.project.config.active_adapters.is_empty()
                && app.modal.is_none()
                && last_poll.elapsed() >= interval
            {
                if let Err(err) = app.sync_pull(adapters_path) {
                    app.status_message = format!("Auto-sync failed: {err}");
                }
                last_poll = Instant::now();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, ComponentKind, GridPos};
    use crate::storage::init_workspace;
    use crate::theme::default_theme;

    fn make_component(id: &str, body: &str) -> Component {
        Component {
            id: id.to_string(),
            name: id.to_string(),
            kind: ComponentKind::Custom,
            parent_id: None,
            description: String::new(),
            relative_path: None,
            position: GridPos::default(),
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: body.to_string(),
        }
    }

    fn make_workspace_app(components: Vec<Component>) -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut project = init_workspace(dir.path(), "Editor Test", "Workspace for editor tests")
            .expect("init workspace");
        project.components = components;
        let theme = default_theme().expect("default theme");
        let app = App::new(dir.path().to_path_buf(), project, theme);
        (dir, app)
    }

    #[test]
    fn default_theme_dirs_includes_project_local_dir() {
        let dirs = default_theme_dirs(Path::new("/tmp/rf_demo"));
        assert_eq!(dirs[0], PathBuf::from("/tmp/rf_demo/.rocket/themes"));
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                assert_eq!(
                    dirs.get(1),
                    Some(&PathBuf::from(home).join(".config/rocket-bench/themes"))
                );
            }
        }
    }

    #[test]
    fn build_tui_app_propagates_custom_theme_dirs() {
        use crate::theme::{list_available_themes, DEFAULT_CRT_AMBER_RON};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Theme Dirs Test", "Propagate theme dirs")
            .expect("init workspace");

        let custom_dir = dir.path().join("custom_themes");
        std::fs::create_dir_all(&custom_dir).expect("create custom themes dir");
        let cobalt_ron = DEFAULT_CRT_AMBER_RON.replace("\"crt_amber\"", "\"cobalt_custom\"");
        std::fs::write(custom_dir.join("cobalt_custom.ron"), cobalt_ron)
            .expect("write cobalt_custom.ron");

        let custom_dirs = vec![custom_dir];
        let app = build_tui_app(dir.path(), &custom_dirs).expect("build_tui_app succeeds");

        assert_eq!(app.theme_dirs, custom_dirs);
        let available = list_available_themes(&app.theme_dirs);
        assert!(available.contains(&"cobalt_custom".to_string()));
    }

    #[test]
    fn parse_editor_command_splits_args_and_falls_back_to_vi() {
        assert_eq!(
            parse_editor_command(Some("code --wait")),
            ("code".to_string(), vec!["--wait".to_string()])
        );
        assert_eq!(
            parse_editor_command(Some("  nvim   -u  NONE  ")),
            (
                "nvim".to_string(),
                vec!["-u".to_string(), "NONE".to_string()]
            )
        );
        assert_eq!(parse_editor_command(None), ("vi".to_string(), Vec::new()));
        assert_eq!(
            parse_editor_command(Some("")),
            ("vi".to_string(), Vec::new())
        );
        assert_eq!(
            parse_editor_command(Some("   \t\n ")),
            ("vi".to_string(), Vec::new())
        );
    }

    #[test]
    fn process_pending_editor_autosaves_edits_and_reloads_markdown() {
        let (dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Initial\n")]);
        let expected_md = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("auth_svc.md");
        assert!(!expected_md.exists());

        app.pending_editor_component = Some("auth_svc".to_string());
        let mut invoked = false;

        process_pending_editor_with(&mut app, |path| {
            invoked = true;
            assert_eq!(
                path,
                std::fs::canonicalize(&expected_md).expect("canonical md")
            );
            std::fs::write(path, "# Updated Blueprint\n\nNew details.\n").map_err(|source| {
                RbError::Io {
                    path: path.to_path_buf(),
                    source,
                }
            })
        });

        assert!(invoked);
        assert!(app.pending_editor_component.is_none());
        let comp = app.project.component("auth_svc").expect("component exists");
        assert_eq!(comp.markdown_body, "# Updated Blueprint\n\nNew details.\n");
    }

    #[test]
    fn process_pending_editor_rejects_invalid_component_id() {
        let (_dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        app.pending_editor_component = Some("../escape".to_string());
        let mut invoked = false;

        process_pending_editor_with(&mut app, |_| {
            invoked = true;
            Ok(())
        });

        assert!(!invoked);
        assert!(app.pending_editor_component.is_none());
        assert!(app.status_message.contains("Invalid component ID"));
    }

    #[test]
    fn process_pending_editor_rejects_unknown_component_id() {
        let (_dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        app.pending_editor_component = Some("missing_comp".to_string());
        let mut invoked = false;

        process_pending_editor_with(&mut app, |_| {
            invoked = true;
            Ok(())
        });

        assert!(!invoked);
        assert!(app.pending_editor_component.is_none());
        assert!(app.status_message.contains("not found"));
    }

    #[cfg(unix)]
    #[test]
    fn process_pending_editor_rejects_symlink_escape() {
        let (dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        let outside = dir.path().join("outside.md");
        std::fs::write(&outside, "# Secret\n").expect("write outside file");

        let link_path = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("auth_svc.md");
        std::os::unix::fs::symlink(&outside, &link_path).expect("create symlink");

        app.pending_editor_component = Some("auth_svc".to_string());
        let mut invoked = false;

        process_pending_editor_with(&mut app, |_| {
            invoked = true;
            Ok(())
        });

        assert!(!invoked);
        assert!(app.pending_editor_component.is_none());
        assert!(app.status_message.contains("escapes"));
    }

    #[test]
    fn process_pending_editor_surfaces_editor_error_without_panicking() {
        let (_dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        app.pending_editor_component = Some("auth_svc".to_string());

        process_pending_editor_with(&mut app, |path| {
            Err(RbError::Io {
                path: path.to_path_buf(),
                source: io::Error::other("simulated editor failure"),
            })
        });

        assert!(app.pending_editor_component.is_none());
        assert!(app.status_message.contains("Editor error for auth_svc"));
        assert!(app.status_message.contains("simulated editor failure"));
    }

    #[test]
    fn process_pending_sync_executes_pull_and_surfaces_errors() {
        let (dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        app.project.config.active_adapters = vec!["mock_tracker".to_string()];

        let adapters_path = dir.path().join("adapters.toml");
        let ok_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"components\":[{\"component_id\":\"auth_svc\",\"tasks\":[{\"external_id\":\"TASK-9\",\"title\":\"Done\",\"status\":\"complete\"}]}]}"']
"#;
        std::fs::write(&adapters_path, ok_toml).expect("write adapters.toml");

        app.pending_sync = Some(SyncRequest::Pull);
        process_pending_sync(&mut app, &adapters_path);

        assert!(app.pending_sync.is_none());
        let comp = app.project.component("auth_svc").expect("component exists");
        assert_eq!(comp.external_tasks.len(), 1);
        assert_eq!(comp.external_tasks[0].external_id, "TASK-9");
        assert!(app.status_message.contains("1 component(s) updated"));

        // Now configure a failing adapter command and verify errors surface cleanly
        let fail_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", "echo simulated adapter failure >&2; exit 1"]
"#;
        std::fs::write(&adapters_path, fail_toml).expect("write failing adapters.toml");

        app.pending_sync = Some(SyncRequest::Pull);
        process_pending_sync(&mut app, &adapters_path);

        assert!(app.pending_sync.is_none());
        assert!(app.status_message.contains("Adapter sync failed"));
        assert!(app.status_message.contains("simulated adapter failure"));
    }
}
