use {
    crate::{
        commands::{CommandExec, config::generate_config},
        config::{ScillaConfig, scilla_config_path},
        context::ScillaContext,
        error::ScillaResult,
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

async fn initialize_config() -> anyhow::Result<ScillaConfig> {
    let config_path = scilla_config_path();
    if !config_path.exists() {
        println!(
            "\n{}",
            style("⚠ No configuration file found!").yellow().bold()
        );
        println!(
            "{}",
            style(format!("Expected location: {}", config_path.display())).cyan()
        );
        println!(
            "{}",
            style("Let's generate a configuration file to get started.\n").cyan()
        );

        generate_config().await?;

        println!(
            "\n{}",
            style("✓ Configuration complete! Starting Scilla...\n")
                .green()
                .bold()
        );
    }

    Ok(ScillaConfig::load()?)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ScillaResult<()> {
    println!(
        "{}",
        style("⚡ Scilla — Hacking Through the Solana Matrix")
            .bold()
            .cyan()
    );

    let config = initialize_config().await?;
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
