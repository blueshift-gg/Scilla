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
    dirs,
    solana_commitment_config::CommitmentLevel,
    std::{fs, path::PathBuf},
};

/// Commands related to configuration like RPC_URL , KEYAPAIR_PATH etc
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Generate,
    Edit,
}

impl ConfigCommand {
    pub fn description(&self) -> &'static str {
        match self {
            ConfigCommand::Show => "Show current config",
            ConfigCommand::Generate => "Generate new config",
            ConfigCommand::Edit => "Edit config",
        }
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

async fn generate_config() -> anyhow::Result<()> {
    println!("\n{}", style("Generate New Config").green().bold());

    // RPC URL with presets
    let rpc_url = loop {
        println!("\n{}", style("Select RPC endpoint:").cyan());
        println!("1. Devnet   ({})", DEVNET_RPC);
        println!("2. Mainnet  ({})", MAINNET_RPC);
        println!("3. Testnet  ({})", TESTNET_RPC);
        println!("4. Custom");

        let choice: String = prompt_data("Enter choice (1-4):")?;

        match choice.as_str() {
            "1" => break DEVNET_RPC.to_string(),
            "2" => break MAINNET_RPC.to_string(),
            "3" => break TESTNET_RPC.to_string(),
            "4" => break prompt_data("Enter RPC URL:")?,
            _ => {
                println!("{}", style("Invalid choice, please try again").red());
                continue;
            }
        };
    };

    // Commitment level
    let commitment_level = loop {
        println!("\n{}", style("Select commitment level:").cyan());
        println!("1. Processed");
        println!("2. Confirmed");
        println!("3. Finalized");

        let commitment_choice: String = prompt_data("Enter choice (1-3):")?;

        match commitment_choice.as_str() {
            "1" => break CommitmentLevel::Processed,
            "2" => break CommitmentLevel::Confirmed,
            "3" => break CommitmentLevel::Finalized,
            _ => {
                println!("{}", style("Invalid choice, please try again").red());
                continue;
            }
        };
    };

    // Keypair path
    let default_keypair = dirs::home_dir()
        .unwrap_or_default()
        .join(".config/solana/id.json");

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

    let config = ScillaConfig {
        rpc_url,
        commitment_level,
        keypair_path,
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

    loop {
        println!("\n{}", style("Select RPC endpoint:").cyan());
        println!("1. Devnet   ({})", DEVNET_RPC);
        println!("2. Mainnet  ({})", MAINNET_RPC);
        println!("3. Testnet  ({})", TESTNET_RPC);
        println!("4. Custom");
        println!("5. Keep current");

        let choice: String = prompt_data("Enter choice (1-5):")?;

        match choice.as_str() {
            "1" => {
                config.rpc_url = DEVNET_RPC.to_string();
                break;
            }
            "2" => {
                config.rpc_url = MAINNET_RPC.to_string();
                break;
            }
            "3" => {
                config.rpc_url = TESTNET_RPC.to_string();
                break;
            }
            "4" => {
                config.rpc_url = prompt_data("Enter RPC URL:")?;
                break;
            }
            "5" => break,
            _ => {
                println!("{}", style("Invalid choice, please try again").red());
                continue;
            }
        };
    }

    // Edit Commitment level
    println!("\n{}", style("Current Commitment Level:").cyan());
    println!("{:?}", config.commitment_level);

    loop {
        println!("\n{}", style("Select commitment level:").cyan());
        println!("1. Processed");
        println!("2. Confirmed");
        println!("3. Finalized");
        println!("4. Keep current");

        let commitment_choice: String = prompt_data("Enter choice (1-4):")?;

        match commitment_choice.as_str() {
            "1" => {
                config.commitment_level = CommitmentLevel::Processed;
                break;
            }
            "2" => {
                config.commitment_level = CommitmentLevel::Confirmed;
                break;
            }
            "3" => {
                config.commitment_level = CommitmentLevel::Finalized;
                break;
            }
            "4" => break,
            _ => {
                println!("{}", style("Invalid choice, please try again").red());
                continue;
            }
        };
    }

    // Edit Keypair path
    println!("\n{}", style("Current Keypair Path:").cyan());
    println!("{}", config.keypair_path.display());

    let edit_keypair = loop {
        let input: String = prompt_data("Edit keypair path? (y/n):")?;
        match input.to_lowercase().as_str() {
            "y" | "yes" => break true,
            "n" | "no" => break false,
            _ => {
                println!("{}", style("Please enter 'y' or 'n'").red());
                continue;
            }
        }
    };

    if edit_keypair {
        let default_keypair = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/solana/id.json");

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
