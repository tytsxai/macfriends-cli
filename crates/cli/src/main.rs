use clap::Parser;
use macfriends::{app, cli::Cli};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_output = cli.json;
    let command_name = cli.command.name();

    match app::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            app::report_failure(command_name, json_output, &error);
            ExitCode::from(app::error_exit_code(&error))
        }
    }
}
