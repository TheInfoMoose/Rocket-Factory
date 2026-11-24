use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = rocket_bench::cli::Cli::parse();
    let adapters_path: Option<PathBuf> = cli
        .adapters_file
        .or_else(rocket_bench::cli::default_adapters_path);

    match cli.command {
        Some(command) => {
            let cmd_adapters_path = adapters_path.as_deref().unwrap_or_else(|| Path::new(""));
            match rocket_bench::cli::execute_command(&cli.path, cmd_adapters_path, command) {
                Ok(output) => {
                    print!("{}", output.stdout);
                    if output.exit_code != 0 {
                        ExitCode::from(u8::try_from(output.exit_code).unwrap_or(1))
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            let theme_dirs = rocket_bench::tui::default_theme_dirs(&cli.path);
            let poll_interval = match std::env::var("RKTB_POLL_INTERVAL_SECS") {
                Ok(raw) => match raw.trim().parse::<u64>() {
                    Ok(secs) if secs > 0 => Some(std::time::Duration::from_secs(secs)),
                    _ => {
                        eprintln!(
                            "warning: ignoring invalid RKTB_POLL_INTERVAL_SECS={raw:?}; expected positive integer seconds"
                        );
                        None
                    }
                },
                Err(std::env::VarError::NotPresent) => None,
                Err(std::env::VarError::NotUnicode(raw)) => {
                    eprintln!(
                        "warning: ignoring invalid RKTB_POLL_INTERVAL_SECS={raw:?}; expected positive integer seconds"
                    );
                    None
                }
            };
            match rocket_bench::tui::run_tui(
                &cli.path,
                &theme_dirs,
                adapters_path.as_deref(),
                poll_interval,
                cli.auto_pull,
            ) {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("error: {err}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
