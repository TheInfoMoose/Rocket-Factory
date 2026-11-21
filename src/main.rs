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
            println!(
                "Rocket Factory TUI — launching in Phase 3 (use `rf --help` for CLI commands)"
            );
            ExitCode::SUCCESS
        }
    }
}
