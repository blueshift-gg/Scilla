use {
    crate::{
        commands::CommandFlow,
        config::ScillaConfig,
        context::ScillaContext,
        error::ScillaResult,
        prompt::{prompt_for_command, prompt_section},
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

    let mut command = prompt_for_command()?;
    ctx.nav().set_root(command.section());

    loop {
        match command.process_command(&mut ctx).await {
            CommandFlow::Process(_) => {
                let current = ctx
                    .nav()
                    .current()
                    .expect("Navigation stack should have root");
                command = prompt_section(&current)?;
            }
            CommandFlow::GoBack => {
                if let Some(parent) = ctx.nav().pop() {
                    command = prompt_section(&parent)?;
                } else {
                    command = prompt_for_command()?;
                    ctx.nav().set_root(command.section());
                }
            }
            CommandFlow::Exit => {
                break;
            }
        }
    }

    Ok(CommandFlow::Exit)
}
