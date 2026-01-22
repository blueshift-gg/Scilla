use {
    crate::{
        commands::{
            Command,
            navigation::{NavigationSection, NavigationTarget},
        },
        error::{ScillaError, ScillaResult},
    },
    commands::CommandFlow,
    config::ScillaConfig,
    console::style,
    context::ScillaContext,
    prompt::prompt_main_section,
    std::process::exit,
};

pub mod cli;
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
    if let Some(command) = cli::parse_from_env() {
        cli::process(command).map_err(ScillaError::from)?;
        exit(0);
    }

    println!(
        "{}",
        style("⚡ Scilla — Hacking Through the Solana Matrix")
            .bold()
            .cyan()
    );

    let config = ScillaConfig::load()?;
    let mut ctx = ScillaContext::try_from(config)?;

    let command = prompt_main_section()?;

    let mut res = command.process_command(&mut ctx).await?;

    loop {
        match res {
            CommandFlow::Processed => {
                let current = ctx
                    .get_nav_context()
                    .current()
                    .expect("navigation stack is empty; expected a root section to exist");
                res = current.prompt_and_process_command(&mut ctx).await?;
            }
            CommandFlow::NavigateTo(target) => match target {
                NavigationTarget::MainSection => {
                    ctx.get_nav_context_mut().reset_navigation_to_main();
                    res = NavigationSection::Main
                        .prompt_and_process_command(&mut ctx)
                        .await?;
                }
                NavigationTarget::PreviousSection => {
                    let previous = ctx
                        .get_nav_context_mut()
                        .pop_and_get_previous()
                        .expect("navigation stack is empty; expected a root section to exist");

                    match previous {
                        NavigationSection::Main => {
                            res = NavigationSection::Main
                                .prompt_and_process_command(&mut ctx)
                                .await?;
                        }
                        _ => res = previous.prompt_and_process_command(&mut ctx).await?,
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
