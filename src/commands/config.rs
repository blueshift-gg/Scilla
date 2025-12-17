use {
    crate::{
        commands::CommandExec,
        config::{ScillaConfig, scilla_config_path},
        constants::{DEVNET_RPC, MAINNET_RPC, TESTNET_RPC},
        error::ScillaResult,
        prompt::prompt_data,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::{Confirm, Select},
    solana_commitment_config::CommitmentLevel,
    std::{fs,fmt, path::PathBuf},
};

/// Commands related to configuration like RPC_URL , KEYAPAIR_PATH etc
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Generate,
    Edit,
    GoBack,
}

impl ConfigCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ConfigCommand::Show => "Displaying current Scilla configuration…",
            ConfigCommand::Generate => "Generating new Scilla configuration…",
            ConfigCommand::Edit => "Editing existing Scilla configuration…",
            ConfigCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for ConfigCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            ConfigCommand::Show => "Show ScillaConfig",
            ConfigCommand::Generate => "Generate ScillaConfig",
            ConfigCommand::Edit => "Edit ScillaConfig",
            ConfigCommand::GoBack => "Go Back",
        };
        write!(f, "{}", command)
    }
}

impl ConfigCommand {
    pub async fn process_command(&self, _ctx: &crate::context::ScillaContext) -> ScillaResult<()> {
        match self {
            ConfigCommand::Show => {
                show_config().await?;
            }
            ConfigCommand::Generate => {
                generate_config().await?;
            }
            ConfigCommand::Edit => {
                edit_config().await?;
            }
            ConfigCommand::GoBack => {
                return Ok(CommandExec::GoBack);
            }
        };

        Ok(CommandExec::Process(()))
    }
}

async fn show_config() -> anyhow::Result<()> {
    let config = ScillaConfig::load()?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("RPC URL"), Cell::new(config.rpc_url)])
        .add_row(vec![
            Cell::new("Commitment Level"),
            Cell::new(format!("{:?}", config.commitment_level)),
        ])
        .add_row(vec![
            Cell::new("Keypair Path"),
            Cell::new(config.keypair_path.display().to_string()),
        ]);

    println!("\n{}", style("CURRENT CONFIG").green().bold());
    println!("{}", table);

    Ok(())
}

pub async fn generate_config() -> anyhow::Result<()> {
    // Check if config already exists
    let config_path = scilla_config_path();
    if config_path.exists() {
        println!(
            "\n{}",
            style("⚠ Config file already exists!").yellow().bold()
        );
        println!(
            "{}",
            style(format!("Location: {}", config_path.display())).cyan()
        );
        println!(
            "{}",
            style("Use the 'Edit' option to modify your existing config.").cyan()
        );
        return Ok(());
    }

    println!("\n{}", style("Generate New Config").green().bold());

    // Ask if user wants to use defaults
    let use_defaults = Confirm::new("Use default config? (Devnet RPC, Confirmed commitment)")
        .with_default(true)
        .prompt()?;

    let config = if use_defaults {
        let mut config = ScillaConfig::default();

        println!("\n{}", style("Using default configuration:").cyan());
        println!("  RPC: {}", config.rpc_url);
        println!("  Commitment: {:?}", config.commitment_level);
        println!("  Default keypair: {}", config.keypair_path.display());

        let keypair_path = loop {
            let keypair_prompt = format!(
                "\nEnter keypair path (press Enter for default: {}): ",
                config.keypair_path.display()
            );
            let keypair_input: String = prompt_data(&keypair_prompt)?;

            let keypair_path = if keypair_input.is_empty() {
                config.keypair_path.clone()
            } else {
                PathBuf::from(keypair_input)
            };

            if !keypair_path.exists() {
                println!(
                    "{}",
                    style(format!(
                        "Keypair file not found at: {}",
                        keypair_path.display()
                    ))
                    .red()
                );
                continue;
            }

            break keypair_path;
        };

        config.keypair_path = keypair_path;
        config
    } else {
        let rpc_options = vec![
            format!("Devnet ({})", DEVNET_RPC),
            format!("Mainnet ({})", MAINNET_RPC),
            format!("Testnet ({})", TESTNET_RPC),
            "Custom".to_string(),
        ];

        let rpc_choice = Select::new("Select RPC endpoint:", rpc_options).prompt()?;

        let rpc_url = match rpc_choice.as_str() {
            s if s.starts_with("Devnet") => DEVNET_RPC.to_string(),
            s if s.starts_with("Mainnet") => MAINNET_RPC.to_string(),
            s if s.starts_with("Testnet") => TESTNET_RPC.to_string(),
            _ => prompt_data("Enter RPC URL:")?,
        };

        let commitment_options = vec!["Processed", "Confirmed", "Finalized"];
        let commitment_choice =
            Select::new("Select commitment level:", commitment_options).prompt()?;

        let commitment_level = match commitment_choice {
            "Processed" => CommitmentLevel::Processed,
            "Confirmed" => CommitmentLevel::Confirmed,
            "Finalized" => CommitmentLevel::Finalized,
            _ => unreachable!(),
        };

        let default_keypair = ScillaConfig::default().keypair_path;

        let keypair_path = loop {
            let keypair_prompt = format!(
                "Enter keypair path (default: {}): ",
                default_keypair.display()
            );
            let keypair_input: String = prompt_data(&keypair_prompt)?;
            let keypair_path = if keypair_input.is_empty() {
                default_keypair.clone()
            } else {
                PathBuf::from(keypair_input)
            };

            if !keypair_path.exists() {
                println!(
                    "{}",
                    style(format!(
                        "Keypair file not found at: {}",
                        keypair_path.display()
                    ))
                    .red()
                );
                continue;
            }

            break keypair_path;
        };

        ScillaConfig {
            rpc_url,
            commitment_level,
            keypair_path,
        }
    };

    // Write config
    let config_path = scilla_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml_string = toml::to_string_pretty(&config)?;
    fs::write(&config_path, toml_string)?;

    println!(
        "\n{}",
        style("✓ Config generated successfully!").green().bold()
    );
    println!(
        "{}",
        style(format!("Saved to: {}", config_path.display())).cyan()
    );

    Ok(())
}

async fn edit_config() -> anyhow::Result<()> {
    let mut config = ScillaConfig::load()?;

    println!("\n{}", style("Edit Config").green().bold());

    // Edit RPC URL
    println!("\n{}", style("Current RPC URL:").cyan());
    println!("{}", config.rpc_url);

    let rpc_options = vec![
        format!("Devnet ({})", DEVNET_RPC),
        format!("Mainnet ({})", MAINNET_RPC),
        format!("Testnet ({})", TESTNET_RPC),
        "Custom".to_string(),
        "Keep current".to_string(),
    ];

    let rpc_choice = Select::new("Select RPC endpoint:", rpc_options).prompt()?;

    match rpc_choice.as_str() {
        s if s.starts_with("Devnet") => config.rpc_url = DEVNET_RPC.to_string(),
        s if s.starts_with("Mainnet") => config.rpc_url = MAINNET_RPC.to_string(),
        s if s.starts_with("Testnet") => config.rpc_url = TESTNET_RPC.to_string(),
        "Custom" => config.rpc_url = prompt_data("Enter RPC URL:")?,
        _ => {}
    }

    println!("\n{}", style("Current Commitment Level:").cyan());
    println!("{:?}", config.commitment_level);

    let commitment_options = vec!["Processed", "Confirmed", "Finalized", "Keep current"];
    let commitment_choice = Select::new("Select commitment level:", commitment_options).prompt()?;

    match commitment_choice {
        "Processed" => config.commitment_level = CommitmentLevel::Processed,
        "Confirmed" => config.commitment_level = CommitmentLevel::Confirmed,
        "Finalized" => config.commitment_level = CommitmentLevel::Finalized,
        _ => {}
    }

    println!("\n{}", style("Current Keypair Path:").cyan());
    println!("{}", config.keypair_path.display());

    let edit_keypair = Confirm::new("Edit keypair path?")
        .with_default(false)
        .prompt()?;

    if edit_keypair {
        let default_keypair = ScillaConfig::default().keypair_path;

        loop {
            let keypair_prompt = format!(
                "Enter new keypair path (default: {}): ",
                default_keypair.display()
            );
            let keypair_input: String = prompt_data(&keypair_prompt)?;
            let keypair_path = if keypair_input.is_empty() {
                default_keypair.clone()
            } else {
                PathBuf::from(keypair_input)
            };

            if !keypair_path.exists() {
                println!(
                    "{}",
                    style(format!(
                        "Keypair file not found at: {}",
                        keypair_path.display()
                    ))
                    .red()
                );
                continue;
            }

            config.keypair_path = keypair_path;
            break;
        }
    }

    // Write updated config
    let config_path = scilla_config_path();
    let toml_string = toml::to_string_pretty(&config)?;
    fs::write(&config_path, toml_string)?;

    println!(
        "\n{}",
        style("✓ Config updated successfully!").green().bold()
    );
    println!(
        "{}",
        style(format!("Saved to: {}", config_path.display())).cyan()
    );

    Ok(())
}
