use {
    crate::{commands::NavigationTarget, error::ScillaResult},
    commands::CommandFlow,
    config::ScillaConfig,
    console::style,
    context::ScillaContext,
    prompt::{prompt_for_command, prompt_section},
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
async fn main() -> ScillaResult {
    println!(
        "{}",
        style("⚡ Scilla — Hacking Through the Solana Matrix")
            .bold()
            .cyan()
    );

    let config = ScillaConfig::load()?;
    let mut ctx = ScillaContext::try_from(config)?;

    let mut command = prompt_for_command()?;
    ctx.get_nav_context_mut().set_root(command.section());

    loop {
        match command.process_command(&mut ctx).await {
            CommandFlow::Processed => {
                let current = ctx
                    .get_nav_context_mut()
                    .current()
                    .expect("Navigation stack should have root");
                command = prompt_section(&current)?;
            }
            CommandFlow::NavigateTo(target) => match target {
                NavigationTarget::MainSection => {
                    command = prompt_for_command()?;
                    ctx.get_nav_context_mut().set_root(command.section());
                }
                NavigationTarget::PreviousSection => {
                    ctx.get_nav_context_mut().pop();
                    let previous = ctx
                        .get_nav_context_mut()
                        .current()
                        .expect("Navigation stack should have root");
                    command = prompt_section(&previous)?;
                }
            },
            CommandFlow::Exit => {
                break;
            }
        }
    }

    Ok(CommandFlow::Exit)
}
