use {
    crate::{commands::navigation::NavigationTarget, context::ScillaContext},
    console::style,
    std::process::{ExitCode, Termination},
};

pub mod account;
pub mod build;
pub mod cluster;
pub mod config;
pub mod main_command;
pub mod navigation;
pub mod program;
pub mod stake;
pub mod transaction;
pub mod vote;

pub enum CommandFlow {
    Processed,
    NavigateTo(NavigationTarget),
    Exit,
}

impl Termination for CommandFlow {
    fn report(self) -> std::process::ExitCode {
        println!("{}", style("Goodbye 👋").dim());
        ExitCode::SUCCESS
    }
}

pub trait Command {
    fn process_command(
        &self,
        ctx: &mut ScillaContext,
    ) -> impl Future<Output = anyhow::Result<CommandFlow>>;
}
