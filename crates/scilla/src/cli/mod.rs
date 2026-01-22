use {
    crate::cli::run,
    std::{env, path::PathBuf},
};

pub mod build;
pub use build::*;

pub enum CliCommand {
    Build(PathBuf),
    Help,
}

pub fn parse_from_env() -> Option<CliCommand> {
    let mut args = env::args().skip(1);
    let subcommand = args.next()?;

    match subcommand.as_str() {
        "build" => {
            let Some(path) = args.next().map(PathBuf::from) else {
                return Some(CliCommand::Help);
            };
            Some(CliCommand::Build(path))
        }
        "help" | "-h" | "--help" => Some(CliCommand::Help),
        _ => Some(CliCommand::Help),
    }
}

pub fn process(command: CliCommand) -> anyhow::Result<()> {
    match command {
        CliCommand::Build(file) => Ok(run(file)?),
        CliCommand::Help => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!("Usage: scilla build [path]");
}
