use {
    crate::{
        commands::CommandExec, config::ScillaConfig, context::ScillaContext, error::ScillaResult,
        prompt::prompt_for_command,
    },
    console::style,
};

pub mod commands;
pub mod config;
pub mod constants;
pub mod context;
pub mod error;
pub mod misc;
pub mod prompt;
pub mod ui;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ScillaResult<()> {
    println!(
        "{}",
        style("⚡ Scilla — Hacking Through the Solana Matrix")
            .bold()
            .cyan()
    );

    let config = ScillaConfig::load()?;
    let ctx = ScillaContext::from_config(config)?;

    loop {
        let command = prompt_for_command()?;

        let res = command.process_command(&ctx).await?;

        match res {
            CommandExec::Process(_) => continue,
            CommandExec::GoBack => continue,
            CommandExec::Exit => break,
        }
    }

    Ok(CommandExec::Exit)
}
