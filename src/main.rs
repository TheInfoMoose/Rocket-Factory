use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = rocket_factory::cli::Cli::parse();
    let adapters_path = cli
        .adapters_file
        .unwrap_or_else(rocket_factory::cli::default_adapters_path);

    match cli.command {
        Some(command) => {
            match rocket_factory::cli::execute_command(&cli.path, &adapters_path, command) {
                Ok(output) => {
                    print!("{}", output.stdout);
                    if output.exit_code != 0 {
                        ExitCode::from(output.exit_code as u8)
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
            let theme_dirs = rocket_factory::tui::default_theme_dirs(&cli.path);
            let poll_interval = std::env::var("RF_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|raw| raw.trim().parse::<u64>().ok())
                .filter(|&secs| secs > 0)
                .map(std::time::Duration::from_secs);
            match rocket_factory::tui::run_tui(
                &cli.path,
                &theme_dirs,
                &adapters_path,
                poll_interval,
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
