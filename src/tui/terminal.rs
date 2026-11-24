use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::error::{RbError, RbResult};
use crate::storage::{
    ensure_contained, save_component, validate_component_id, COMPONENTS_DIR, ROCKET_DIR,
};
use crate::tui::app::{apply_sync_outcome, run_sync_job, App, SyncOutcome, SyncRequest};
use crate::tui::ui;

pub(crate) type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

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

pub(crate) fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

pub(crate) fn enter_terminal() -> RbResult<TuiTerminal> {
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

pub(crate) fn restore_terminal(terminal: &mut TuiTerminal) -> RbResult<()> {
    let mut first_err: Option<RbError> = None;
    if let Err(err) = disable_raw_mode() {
        first_err = Some(terminal_io_err(err));
    }
    if let Err(err) = execute!(terminal.backend_mut(), LeaveAlternateScreen) {
        if first_err.is_none() {
            first_err = Some(terminal_io_err(err));
        }
    }
    if let Err(err) = terminal.show_cursor() {
        if first_err.is_none() {
            first_err = Some(terminal_io_err(err));
        }
    }
    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

fn combine_tui_results(loop_result: RbResult<()>, restore_result: RbResult<()>) -> RbResult<()> {
    match (loop_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(loop_err), Ok(())) => Err(loop_err),
        (Ok(()), Err(restore_err)) => Err(restore_err),
        (Err(loop_err), Err(restore_err)) => Err(RbError::Io {
            path: PathBuf::from("<terminal>"),
            source: io::Error::other(format!(
                "TUI loop failed ({loop_err}); terminal restore also failed ({restore_err})"
            )),
        }),
    }
}

fn parse_editor_command(editor_value: Option<&str>) -> (String, Vec<String>) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum QuoteState {
        None,
        Single,
        Double,
    }

    let Some(raw) = editor_value else {
        return ("vi".to_string(), Vec::new());
    };

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut state = QuoteState::None;
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        match state {
            QuoteState::None => match c {
                c if c.is_ascii_whitespace() => {
                    if in_token {
                        tokens.push(std::mem::take(&mut current));
                        in_token = false;
                    }
                }
                '\'' => {
                    state = QuoteState::Single;
                    in_token = true;
                }
                '"' => {
                    state = QuoteState::Double;
                    in_token = true;
                }
                '\\' => {
                    in_token = true;
                    if let Some(next) = chars.next() {
                        current.push(next);
                    } else {
                        current.push('\\');
                    }
                }
                other => {
                    in_token = true;
                    current.push(other);
                }
            },
            QuoteState::Single => {
                if c == '\'' {
                    state = QuoteState::None;
                } else {
                    current.push(c);
                }
            }
            QuoteState::Double => match c {
                '"' => {
                    state = QuoteState::None;
                }
                '\\' => {
                    if let Some(&next) = chars.peek() {
                        if next == '"' || next == '\\' {
                            chars.next();
                            current.push(next);
                        } else {
                            current.push('\\');
                        }
                    } else {
                        current.push('\\');
                    }
                }
                other => {
                    current.push(other);
                }
            },
        }
    }

    if in_token {
        tokens.push(current);
    }

    let mut iter = tokens.into_iter();
    if let Some(program) = iter.next() {
        if !program.is_empty() {
            return (program, iter.collect());
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
    if let Err(err) = execute!(terminal.backend_mut(), EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(terminal_io_err(err));
    }
    if let Err(err) = terminal.clear() {
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        return Err(terminal_io_err(err));
    }

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

    let canon_md = match ensure_contained(&components_dir, &md_path) {
        Ok(path) => path,
        Err(RbError::PathEscape(_)) => {
            app.status_message = format!("Refusing to edit {comp_id}: path escapes components dir");
            return;
        }
        Err(err) => {
            app.status_message = format!("Failed to resolve Markdown path for {comp_id}: {err}");
            return;
        }
    };

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

pub(crate) fn should_startup_pull(
    auto_pull: bool,
    active_adapters: &[String],
    adapters_path: Option<&Path>,
) -> bool {
    auto_pull && !active_adapters.is_empty() && adapters_path.is_some()
}

fn spawn_sync_request(
    app: &mut App,
    request: SyncRequest,
    adapters_path: Option<&Path>,
    is_auto: bool,
    tx: &mpsc::Sender<SyncOutcome>,
) -> bool {
    if app.sync_in_flight {
        app.status_message = "Adapter sync already in progress".to_string();
        return false;
    }

    let Some(path) = adapters_path else {
        app.status_message =
            "No adapters file configured (set HOME or pass --adapters-file)".to_string();
        return false;
    };

    if app.project.config.active_adapters.is_empty() {
        if !is_auto {
            app.status_message = "No active adapters configured".to_string();
        }
        return false;
    }

    let target_comp_id = match request {
        SyncRequest::Pull => None,
        SyncRequest::PushSelected => match app.selected_component().map(|c| c.id.clone()) {
            Some(id) => Some(id),
            None => {
                app.status_message = "No component selected to push".to_string();
                return false;
            }
        },
    };

    app.sync_in_flight = true;
    let project_snapshot = app.project.clone();
    let root = app.root.clone();
    let adapters_path = path.to_path_buf();
    let tx = tx.clone();

    std::thread::spawn(move || {
        let outcome = run_sync_job(
            project_snapshot,
            &root,
            &adapters_path,
            request,
            target_comp_id.as_deref(),
            is_auto,
        );
        let _ = tx.send(outcome);
    });

    true
}

fn process_pending_sync(
    app: &mut App,
    adapters_path: Option<&Path>,
    tx: &mpsc::Sender<SyncOutcome>,
) -> bool {
    let Some(request) = app.pending_sync.take() else {
        return false;
    };
    spawn_sync_request(app, request, adapters_path, false, tx);
    true
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
    adapters_path: Option<&Path>,
    poll_interval: Option<Duration>,
    auto_pull: bool,
) -> RbResult<()> {
    let mut app = build_tui_app(root, theme_dirs)?;

    if should_startup_pull(
        auto_pull,
        &app.project.config.active_adapters,
        adapters_path,
    ) {
        if let Some(path) = adapters_path {
            if let Err(err) = app.sync_pull(path) {
                app.status_message = format!("Initial adapter sync failed: {err}");
            }
        }
    }

    install_panic_hook();
    let mut terminal = enter_terminal()?;
    let loop_result = run_event_loop(&mut terminal, &mut app, adapters_path, poll_interval);
    let restore_result = restore_terminal(&mut terminal);

    combine_tui_results(loop_result, restore_result)
}

struct EventLoopStepCtx<'a> {
    adapters_path: Option<&'a Path>,
    poll_interval: Option<Duration>,
    last_poll: &'a mut Instant,
    sync_tx: &'a mpsc::Sender<SyncOutcome>,
    sync_rx: &'a mpsc::Receiver<SyncOutcome>,
}

fn handle_event(app: &mut App, ev: Event) -> bool {
    match ev {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            app.handle_key(key);
            true
        }
        Event::Resize(_, _) => true,
        _ => false,
    }
}

fn step_event_loop<FNext, FEditor>(
    app: &mut App,
    ctx: &mut EventLoopStepCtx<'_>,
    mut next_event: FNext,
    mut run_editor: FEditor,
) -> RbResult<bool>
where
    FNext: FnMut(Duration) -> RbResult<Option<Event>>,
    FEditor: FnMut(&Path) -> RbResult<()>,
{
    let mut dirty = false;

    while let Ok(outcome) = ctx.sync_rx.try_recv() {
        apply_sync_outcome(app, outcome);
        dirty = true;
    }

    let first_timeout = if dirty {
        Duration::ZERO
    } else {
        Duration::from_millis(100)
    };
    if let Some(first) = next_event(first_timeout)? {
        if handle_event(app, first) {
            dirty = true;
        }
        while !app.should_quit && app.pending_editor_component.is_none() {
            let Some(next) = next_event(Duration::ZERO)? else {
                break;
            };
            if handle_event(app, next) {
                dirty = true;
            }
        }
    }

    if app.should_quit {
        return Ok(dirty);
    }

    if app.pending_editor_component.is_some() {
        process_pending_editor_with(app, |path| run_editor(path));
        dirty = true;
    }

    if process_pending_sync(app, ctx.adapters_path, ctx.sync_tx) {
        dirty = true;
    }

    while let Ok(outcome) = ctx.sync_rx.try_recv() {
        apply_sync_outcome(app, outcome);
        dirty = true;
    }

    if let (Some(interval), Some(path)) = (ctx.poll_interval, ctx.adapters_path) {
        if !app.project.config.active_adapters.is_empty()
            && app.modal.is_none()
            && !app.sync_in_flight
            && ctx.last_poll.elapsed() >= interval
        {
            if spawn_sync_request(app, SyncRequest::Pull, Some(path), true, ctx.sync_tx) {
                dirty = true;
            }
            *ctx.last_poll = Instant::now();
        }
    }

    Ok(dirty)
}

fn run_event_loop(
    terminal: &mut TuiTerminal,
    app: &mut App,
    adapters_path: Option<&Path>,
    poll_interval: Option<Duration>,
) -> RbResult<()> {
    let mut last_poll = Instant::now();
    let (sync_tx, sync_rx) = mpsc::channel();
    let mut dirty = true;

    loop {
        if dirty {
            terminal
                .draw(|f| ui::render(f, app))
                .map_err(terminal_io_err)?;
            dirty = false;
        }

        if app.should_quit {
            break;
        }

        let mut ctx = EventLoopStepCtx {
            adapters_path,
            poll_interval,
            last_poll: &mut last_poll,
            sync_tx: &sync_tx,
            sync_rx: &sync_rx,
        };
        if step_event_loop(
            app,
            &mut ctx,
            |timeout| {
                if event::poll(timeout).map_err(terminal_io_err)? {
                    event::read().map(Some).map_err(terminal_io_err)
                } else {
                    Ok(None)
                }
            },
            |path| suspend_for_editor(terminal, path),
        )? {
            dirty = true;
        }

        if app.should_quit {
            break;
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
    fn parse_editor_command_preserves_quoted_paths_with_spaces() {
        assert_eq!(
            parse_editor_command(Some("\"/path/with spaces/code\" --wait")),
            (
                "/path/with spaces/code".to_string(),
                vec!["--wait".to_string()]
            )
        );
        assert_eq!(
            parse_editor_command(Some("'/opt/my editor/bin' -n")),
            ("/opt/my editor/bin".to_string(), vec!["-n".to_string()])
        );
        assert_eq!(
            parse_editor_command(Some("/opt/my\\ editor/bin --flag=\"a b\"")),
            (
                "/opt/my editor/bin".to_string(),
                vec!["--flag=a b".to_string()]
            )
        );
        assert_eq!(
            parse_editor_command(Some("editor --msg=\"say \\\"hi\\\\\\\"\" 'keep \\\"raw\"'")),
            (
                "editor".to_string(),
                vec![
                    "--msg=say \"hi\\\"".to_string(),
                    "keep \\\"raw\"".to_string()
                ]
            )
        );
    }

    #[test]
    fn parse_editor_command_handles_unclosed_quotes_and_defaults() {
        assert_eq!(parse_editor_command(None), ("vi".to_string(), Vec::new()));
        assert_eq!(
            parse_editor_command(Some("   ")),
            ("vi".to_string(), Vec::new())
        );
        assert_eq!(
            parse_editor_command(Some("\"/path/with spaces/code")),
            ("/path/with spaces/code".to_string(), Vec::new())
        );
        assert_eq!(
            parse_editor_command(Some("'/opt/unclosed editor")),
            ("/opt/unclosed editor".to_string(), Vec::new())
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

        let (tx, rx) = mpsc::channel();
        app.pending_sync = Some(SyncRequest::Pull);
        assert!(process_pending_sync(&mut app, Some(&adapters_path), &tx));
        assert!(app.pending_sync.is_none());
        assert!(app.sync_in_flight);

        let outcome = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("receive pull outcome");
        apply_sync_outcome(&mut app, outcome);

        assert!(!app.sync_in_flight);
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
        assert!(process_pending_sync(&mut app, Some(&adapters_path), &tx));
        assert!(app.pending_sync.is_none());
        assert!(app.sync_in_flight);

        let fail_outcome = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("receive fail outcome");
        apply_sync_outcome(&mut app, fail_outcome);

        assert!(!app.sync_in_flight);
        assert!(app.status_message.contains("Adapter sync failed"));
        assert!(app.status_message.contains("simulated adapter failure"));
    }

    #[test]
    fn startup_pull_requires_auto_pull_flag_and_adapters_path() {
        let active = vec!["mock_tracker".to_string()];
        let empty: Vec<String> = Vec::new();
        let path = Path::new("/home/testuser/.config/rocket-bench/adapters.toml");

        assert!(!should_startup_pull(false, &active, Some(path)));
        assert!(!should_startup_pull(true, &active, None));
        assert!(!should_startup_pull(true, &empty, Some(path)));
        assert!(should_startup_pull(true, &active, Some(path)));

        let (_dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        let (tx, _rx) = mpsc::channel();
        app.pending_sync = Some(SyncRequest::Pull);
        assert!(process_pending_sync(&mut app, None, &tx));

        assert!(app.pending_sync.is_none());
        assert!(!app.sync_in_flight);
        assert_eq!(
            app.status_message,
            "No adapters file configured (set HOME or pass --adapters-file)"
        );
    }

    #[test]
    fn combine_tui_results_preserves_both_loop_and_restore_errors() {
        assert!(combine_tui_results(Ok(()), Ok(())).is_ok());

        let only_loop = combine_tui_results(
            Err(RbError::Io {
                path: PathBuf::from("<terminal>"),
                source: io::Error::other("loop read failed"),
            }),
            Ok(()),
        )
        .expect_err("loop error returned");
        assert!(only_loop.to_string().contains("loop read failed"));

        let only_restore = combine_tui_results(
            Ok(()),
            Err(RbError::Io {
                path: PathBuf::from("<terminal>"),
                source: io::Error::other("restore raw mode failed"),
            }),
        )
        .expect_err("restore error returned");
        assert!(only_restore.to_string().contains("restore raw mode failed"));

        let both = combine_tui_results(
            Err(RbError::Io {
                path: PathBuf::from("<terminal>"),
                source: io::Error::other("loop draw crashed"),
            }),
            Err(RbError::Io {
                path: PathBuf::from("<terminal>"),
                source: io::Error::other("leave alt screen failed"),
            }),
        )
        .expect_err("combined error returned");
        let msg = both.to_string();
        assert!(msg.contains("loop draw crashed"), "missing loop err: {msg}");
        assert!(
            msg.contains("leave alt screen failed"),
            "missing restore err: {msg}"
        );
    }

    #[test]
    fn async_sync_channel_applies_pull_outcome_without_blocking() {
        let (dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        app.project.config.active_adapters = vec!["mock_tracker".to_string()];

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", 'sleep 0.1; printf "%s" "{\"components\":[{\"component_id\":\"auth_svc\",\"tasks\":[{\"external_id\":\"TASK-77\",\"title\":\"Async pulled\",\"status\":\"in_progress\"}]}]}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        let (tx, rx) = mpsc::channel();
        app.pending_sync = Some(SyncRequest::Pull);
        assert!(process_pending_sync(&mut app, Some(&adapters_path), &tx));
        assert!(app.sync_in_flight);

        // Concurrent second sync request is rejected without spawning another worker
        app.pending_sync = Some(SyncRequest::Pull);
        assert!(process_pending_sync(&mut app, Some(&adapters_path), &tx));
        assert!(app.sync_in_flight);
        assert_eq!(app.status_message, "Adapter sync already in progress");

        // Unrelated in-memory UI edit while sync is in flight
        if let Some(comp) = app.project.component_mut("auth_svc") {
            comp.description = "Edited while sync in flight".to_string();
        }

        let outcome = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("receive async pull outcome");
        apply_sync_outcome(&mut app, outcome);

        assert!(!app.sync_in_flight);
        let comp = app.project.component("auth_svc").expect("component exists");
        assert_eq!(comp.external_tasks.len(), 1);
        assert_eq!(comp.external_tasks[0].external_id, "TASK-77");
        assert_eq!(comp.description, "Edited while sync in flight");
        assert!(app.status_message.contains("1 component(s) updated"));
    }

    #[test]
    fn event_loop_dirty_flag_skips_idle_redraws_and_drains_burst_events() {
        use std::collections::VecDeque;

        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let (_dir, mut app) = make_workspace_app(vec![
            make_component("comp_0", ""),
            make_component("comp_1", ""),
            make_component("comp_2", ""),
            make_component("comp_3", ""),
        ]);

        let (sync_tx, sync_rx) = mpsc::channel();
        let mut last_poll = Instant::now();
        let mut queue: VecDeque<Event> = VecDeque::new();

        // 1. Idle timeout (empty poll) -> dirty stays false (no redraw)
        let mut ctx = EventLoopStepCtx {
            adapters_path: None,
            poll_interval: None,
            last_poll: &mut last_poll,
            sync_tx: &sync_tx,
            sync_rx: &sync_rx,
        };
        let dirty_idle = step_event_loop(
            &mut app,
            &mut ctx,
            |_timeout| Ok(queue.pop_front()),
            |_| Ok(()),
        )
        .expect("idle step succeeds");
        assert!(!dirty_idle);
        assert_eq!(app.selected_index, 0);

        // 2. Burst of 3 queued key events -> all 3 processed in one batch with a single dirty = true
        for _ in 0..3 {
            queue.push_back(Event::Key(KeyEvent::new(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
            )));
        }
        let dirty_burst = step_event_loop(
            &mut app,
            &mut ctx,
            |_timeout| Ok(queue.pop_front()),
            |_| Ok(()),
        )
        .expect("burst step succeeds");
        assert!(dirty_burst);
        assert!(queue.is_empty());
        assert_eq!(app.selected_index, 3);

        // 3. Resize event -> dirty becomes true
        queue.push_back(Event::Resize(120, 40));
        let dirty_resize = step_event_loop(
            &mut app,
            &mut ctx,
            |_timeout| Ok(queue.pop_front()),
            |_| Ok(()),
        )
        .expect("resize step succeeds");
        assert!(dirty_resize);
        assert!(queue.is_empty());
    }

    #[test]
    fn async_sync_does_not_overwrite_concurrent_foreground_disk_edits() {
        use crate::storage::{delete_component, load_project};

        let mut comp_a = make_component("comp_a", "# Comp A\n");
        comp_a.description = "Initial summary".to_string();
        comp_a.position = GridPos { col: 0, row: 0 };

        let mut comp_b = make_component("comp_b", "# Comp B\n");
        comp_b.description = "To be deleted".to_string();
        comp_b.position = GridPos { col: 1, row: 0 };

        let (dir, mut app) = make_workspace_app(vec![comp_a, comp_b]);
        app.project.config.active_adapters = vec!["mock_tracker".to_string()];
        for comp in &app.project.components {
            save_component(&app.root, comp).expect("save initial component");
        }

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"components\":[{\"component_id\":\"comp_a\",\"tasks\":[{\"external_id\":\"TASK-A1\",\"title\":\"Pulled A\",\"status\":\"complete\"}]},{\"component_id\":\"comp_b\",\"tasks\":[{\"external_id\":\"TASK-B1\",\"title\":\"Pulled B\",\"status\":\"in_progress\"}]}]}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        // Snapshot taken before concurrent foreground mutations
        let project_snapshot = app.project.clone();

        // Concurrent foreground edit to comp_a (both in memory and on disk)
        {
            let comp_a_live = app.project.component_mut("comp_a").expect("comp_a exists");
            comp_a_live.description = "Foreground edited summary".to_string();
            comp_a_live.position = GridPos { col: 3, row: 2 };
            save_component(&app.root, comp_a_live).expect("save edited comp_a");
        }

        // Concurrent foreground deletion of comp_b (both in memory and on disk)
        app.project.components.retain(|c| c.id != "comp_b");
        delete_component(&app.root, "comp_b").expect("delete comp_b from disk");

        let comp_b_toml = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("comp_b.toml");
        assert!(!comp_b_toml.exists());

        // Background worker runs on the stale snapshot
        app.sync_in_flight = true;
        let outcome = run_sync_job(
            project_snapshot,
            &app.root,
            &adapters_path,
            SyncRequest::Pull,
            None,
            false,
        );

        // Background worker must not have recreated comp_b.toml or overwritten comp_a on disk
        assert!(!comp_b_toml.exists());

        // UI thread applies outcome and persists live components
        apply_sync_outcome(&mut app, outcome);

        assert!(!app.sync_in_flight);
        assert!(!comp_b_toml.exists());
        assert!(app.project.component("comp_b").is_none());

        let comp_a_mem = app.project.component("comp_a").expect("comp_a in memory");
        assert_eq!(comp_a_mem.description, "Foreground edited summary");
        assert_eq!(comp_a_mem.position, GridPos { col: 3, row: 2 });
        assert_eq!(comp_a_mem.external_tasks.len(), 1);
        assert_eq!(comp_a_mem.external_tasks[0].external_id, "TASK-A1");

        let reloaded = load_project(&app.root).expect("reload project from disk");
        assert!(reloaded.component("comp_b").is_none());
        let comp_a_disk = reloaded.component("comp_a").expect("comp_a on disk");
        assert_eq!(comp_a_disk.description, "Foreground edited summary");
        assert_eq!(comp_a_disk.position, GridPos { col: 3, row: 2 });
        assert_eq!(comp_a_disk.external_tasks.len(), 1);
        assert_eq!(comp_a_disk.external_tasks[0].external_id, "TASK-A1");
    }

    #[test]
    fn step_event_loop_returns_immediately_when_sync_outcome_ready() {
        let (_dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Body\n")]);
        app.sync_in_flight = true;

        let (sync_tx, sync_rx) = mpsc::channel();
        sync_tx
            .send(SyncOutcome::Success {
                updates: Vec::new(),
                status_message: "Synced 1 adapter(s): 0 component(s) updated".to_string(),
            })
            .expect("send sync outcome");

        let mut last_poll = Instant::now();
        let mut ctx = EventLoopStepCtx {
            adapters_path: None,
            poll_interval: None,
            last_poll: &mut last_poll,
            sync_tx: &sync_tx,
            sync_rx: &sync_rx,
        };

        let mut requested_timeouts = Vec::new();
        let dirty = step_event_loop(
            &mut app,
            &mut ctx,
            |timeout| {
                requested_timeouts.push(timeout);
                Ok(None)
            },
            |_| Ok(()),
        )
        .expect("step_event_loop succeeds");

        assert!(dirty);
        assert_eq!(requested_timeouts, vec![Duration::ZERO]);
        assert!(!app.sync_in_flight);
        assert_eq!(
            app.status_message,
            "Synced 1 adapter(s): 0 component(s) updated"
        );
    }

    #[test]
    fn handle_event_filters_key_kind_and_accepts_resize() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventState, KeyModifiers};

        let (_dir, mut app) = make_workspace_app(vec![
            make_component("comp_0", ""),
            make_component("comp_1", ""),
        ]);

        let press =
            KeyEvent::new_with_kind(KeyCode::Char('j'), KeyModifiers::NONE, KeyEventKind::Press);
        assert!(handle_event(&mut app, Event::Key(press)));
        assert_eq!(app.selected_index, 1);

        let release = KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        assert!(!handle_event(&mut app, Event::Key(release)));
        assert_eq!(app.selected_index, 1);

        let repeat = KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Repeat,
            state: KeyEventState::NONE,
        };
        assert!(!handle_event(&mut app, Event::Key(repeat)));
        assert_eq!(app.selected_index, 1);

        assert!(handle_event(&mut app, Event::Resize(100, 30)));
        assert!(!handle_event(&mut app, Event::FocusGained));
    }

    #[test]
    fn step_event_loop_triggers_auto_poll_and_editor_dispatch() {
        let (dir, mut app) = make_workspace_app(vec![make_component("auth_svc", "# Initial\n")]);
        app.project.config.active_adapters = vec!["mock_tracker".to_string()];

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"components\":[]}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        app.pending_editor_component = Some("auth_svc".to_string());

        let (sync_tx, sync_rx) = mpsc::channel();
        let poll_interval = Duration::from_millis(10);
        let mut last_poll = match Instant::now().checked_sub(Duration::from_millis(50)) {
            Some(past) => past,
            None => {
                let now = Instant::now();
                std::thread::sleep(Duration::from_millis(15));
                now
            }
        };

        let mut ctx = EventLoopStepCtx {
            adapters_path: Some(&adapters_path),
            poll_interval: Some(poll_interval),
            last_poll: &mut last_poll,
            sync_tx: &sync_tx,
            sync_rx: &sync_rx,
        };

        let mut editor_invoked = false;
        let dirty = step_event_loop(
            &mut app,
            &mut ctx,
            |_timeout| Ok(None),
            |path| {
                editor_invoked = true;
                std::fs::write(path, "# Edited via loop\n").map_err(|source| RbError::Io {
                    path: path.to_path_buf(),
                    source,
                })
            },
        )
        .expect("step_event_loop succeeds");

        assert!(dirty);
        assert!(editor_invoked);
        assert!(app.pending_editor_component.is_none());
        let comp = app.project.component("auth_svc").expect("component exists");
        assert_eq!(comp.markdown_body, "# Edited via loop\n");
        assert!(app.sync_in_flight);

        // Drain the background sync thread before tempdir cleanup.
        let outcome = sync_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("receive auto-poll outcome");
        apply_sync_outcome(&mut app, outcome);
        assert!(!app.sync_in_flight);
    }

    #[test]
    fn build_tui_app_returns_not_found_on_uninitialized_dir_and_combine_results() {
        let empty_dir = tempfile::tempdir().expect("tempdir");
        let err = match build_tui_app(empty_dir.path(), &[]) {
            Ok(_) => panic!("expected NotFound on uninitialized directory"),
            Err(err) => err,
        };
        assert!(matches!(err, RbError::NotFound(_)));

        assert!(combine_tui_results(Ok(()), Ok(())).is_ok());
        assert!(matches!(
            combine_tui_results(
                Err(RbError::NotFound("loop".to_string())),
                Ok(()),
            ),
            Err(RbError::NotFound(msg)) if msg == "loop"
        ));
        assert!(matches!(
            combine_tui_results(
                Ok(()),
                Err(RbError::NotFound("restore".to_string())),
            ),
            Err(RbError::NotFound(msg)) if msg == "restore"
        ));
    }
}
