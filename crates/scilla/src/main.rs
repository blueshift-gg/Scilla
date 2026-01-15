use {
    crate::{commands::NavigationTarget, error::ScillaResult, prompt::prompt_go_back},
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
    ctx.nav().set_root(command.section());

    loop {
        match command.process_command(&mut ctx).await {
            CommandFlow::Processed => {
                let current = ctx
                    .nav()
                    .current()
                    .expect("Navigation stack should have root");
                command = prompt_section(&current)?;
            }
            CommandFlow::NavigateTo(target) => match target {
                NavigationTarget::MainMenu => {
                    command = prompt_for_command()?;
                    ctx.nav().set_root(command.section());
                }
                NavigationTarget::PreviousSection => {
                    if ctx.nav().is_nested() {
                        match prompt_go_back() {
                            NavigationTarget::MainMenu => {
                                command = prompt_for_command()?;
                                ctx.nav().set_root(command.section());
                            }
                            NavigationTarget::PreviousSection => {
                                ctx.nav().pop();
                                let previous = ctx
                                    .nav()
                                    .current()
                                    .expect("Navigation stack should have root");
                                command = prompt_section(&previous)?;
                            }
                        }
                    } else {
                        command = prompt_for_command()?;
                        ctx.nav().set_root(command.section());
                    }
                }
            },
            CommandFlow::Exit => {
                break;
            }
        }
    }

    Ok(CommandFlow::Exit)
}
