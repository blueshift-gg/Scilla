use {
    crate::context::ScillaContext,
    console::style,
    std::process::{ExitCode, Termination},
};

pub mod account;
pub mod cluster;
pub mod config;
pub mod main_command;
pub mod navigation;
pub mod program;
pub mod stake;
pub mod transaction;
pub mod vote;

pub use navigation::NavigationTarget;

pub trait Command {
    fn process_command(
        &self,
        ctx: &mut ScillaContext,
    ) -> impl std::future::Future<Output = anyhow::Result<CommandFlow>>;
}

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
