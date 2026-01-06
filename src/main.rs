use {
    crate::{
        commands::CommandFlow, config::ScillaConfig, context::ScillaContext, error::ScillaResult,
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
    let mut ctx = ScillaContext::try_from(config)?;

    loop {
        let command = prompt_for_command()?;

        let res = command.process_command(&mut ctx).await;

        match res {
            CommandFlow::Process(_) => continue,
            CommandFlow::GoBack => continue,
            CommandFlow::Exit => break,
        }
    }

    Ok(CommandFlow::Exit)
}
