use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::error::{RfError, RfResult};
use crate::tui::app::App;
use crate::tui::ui;

pub type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

fn terminal_io_err(source: io::Error) -> RfError {
    RfError::Io {
        path: PathBuf::from("<terminal>"),
        source,
    }
}

pub fn default_theme_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.join(".rocket/themes")];
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            dirs.push(PathBuf::from(home).join(".config/rocket-factory/themes"));
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

pub fn enter_terminal() -> RfResult<TuiTerminal> {
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

pub fn restore_terminal(terminal: &mut TuiTerminal) -> RfResult<()> {
    disable_raw_mode().map_err(terminal_io_err)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(terminal_io_err)?;
    terminal.show_cursor().map_err(terminal_io_err)?;
    Ok(())
}

pub fn suspend_for_editor(terminal: &mut TuiTerminal, file_path: &Path) -> RfResult<()> {
    restore_terminal(terminal)?;

    let editor = std::env::var("EDITOR")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "vi".to_string());

    let status_res = Command::new(&editor).arg(file_path).status();

    enable_raw_mode().map_err(terminal_io_err)?;
    execute!(terminal.backend_mut(), EnterAlternateScreen).map_err(terminal_io_err)?;
    terminal.clear().map_err(terminal_io_err)?;

    let status = status_res.map_err(|source| RfError::Io {
        path: file_path.to_path_buf(),
        source,
    })?;

    if !status.success() {
        return Err(RfError::Io {
            path: file_path.to_path_buf(),
            source: io::Error::other(format!("editor '{editor}' exited with status {status}")),
        });
    }

    Ok(())
}

pub fn run_tui(root: &Path, theme_dirs: &[PathBuf]) -> RfResult<()> {
    let project = crate::storage::load_project(root)?;
    let theme = crate::theme::resolve_theme(&project.config.theme, theme_dirs)?;
    let mut app = App::new(root.to_path_buf(), project, theme);

    install_panic_hook();
    let mut terminal = enter_terminal()?;
    let loop_result = run_event_loop(&mut terminal, &mut app);
    let restore_result = restore_terminal(&mut terminal);

    loop_result.and(restore_result)
}

fn run_event_loop(terminal: &mut TuiTerminal, app: &mut App) -> RfResult<()> {
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
                }
            }
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

    #[test]
    fn default_theme_dirs_includes_project_local_dir() {
        let dirs = default_theme_dirs(Path::new("/tmp/rf_demo"));
        assert_eq!(dirs[0], PathBuf::from("/tmp/rf_demo/.rocket/themes"));
    }
}
