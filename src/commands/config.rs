use {
    crate::{
        commands::CommandExec,
        config::{ScillaConfig, scilla_config_path},
        error::ScillaResult,
        prompt::prompt_data,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::{Confirm, Select},
    solana_commitment_config::CommitmentLevel,
    std::{fmt, fs, path::PathBuf},
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
            ConfigCommand::Show => "View ScillaConfig",
            ConfigCommand::Generate => "Generate ScillaConfig",
            ConfigCommand::Edit => "Edit ScillaConfig",
            ConfigCommand::GoBack => "Go back",
        };
        write!(f, "{command}")
    }
}

#[derive(Debug, Clone)]
enum ConfigField {
    RpcUrl,
    CommitmentLevel,
    KeypairPath,
}

impl fmt::Display for ConfigField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigField::RpcUrl => write!(f, "RPC URL"),
            ConfigField::CommitmentLevel => write!(f, "Commitment Level"),
            ConfigField::KeypairPath => write!(f, "Keypair Path"),
        }
    }
}

impl ConfigField {
    fn all() -> Vec<Self> {
        vec![
            ConfigField::RpcUrl,
            ConfigField::CommitmentLevel,
            ConfigField::KeypairPath,
        ]
    }
}

fn get_commitment_levels() -> Vec<CommitmentLevel> {
    vec![
        CommitmentLevel::Processed,
        CommitmentLevel::Confirmed,
        CommitmentLevel::Finalized,
    ]
}

impl ConfigCommand {
    pub fn process_command(&self) -> ScillaResult<()> {
        match self {
            ConfigCommand::Show => {
                show_config()?;
            }
            ConfigCommand::Generate => {
                generate_config()?;
            }
            ConfigCommand::Edit => {
                edit_config()?;
            }
            ConfigCommand::GoBack => return Ok(CommandExec::GoBack),
        };

        Ok(CommandExec::Process(()))
    }
}

fn show_config() -> anyhow::Result<()> {
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
            Cell::new(config.commitment_level.to_string()),
        ])
        .add_row(vec![
            Cell::new("Keypair Path"),
            Cell::new(config.keypair_path.display().to_string()),
        ]);

    println!("\n{}", style("SCILLA CONFIG").green().bold());
    println!("{}", table);

    Ok(())
}

pub fn generate_config() -> anyhow::Result<()> {
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
        let config = ScillaConfig::default();

        println!("\n{}", style("Using default configuration:").cyan());
        println!("  RPC: {}", config.rpc_url);
        println!("  Commitment: {:?}", config.commitment_level);
        println!("  Keypair: {}", config.keypair_path.display());

        config
    } else {
        let rpc_url: String = prompt_data("Enter RPC URL:")?;

        let commitment_level =
            Select::new("Select commitment level:", get_commitment_levels()).prompt()?;

        let default_keypair_path = ScillaConfig::default().keypair_path;

        let keypair_path = loop {
            let keypair_input: PathBuf = prompt_data(&format!(
                "Enter keypair path (press Enter to use default: {}): ",
                default_keypair_path.display()
            ))?;

            if keypair_input.as_os_str().is_empty() {
                break default_keypair_path.clone();
            }

            if !keypair_input.exists() {
                println!(
                    "{}",
                    style(format!(
                        "Keypair file not found at: {}",
                        keypair_input.display()
                    ))
                    .red()
                );
                continue;
            }

            break keypair_input;
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

fn edit_config() -> anyhow::Result<()> {
    let mut config = ScillaConfig::load()?;

    println!("\n{}", style("Edit Config").green().bold());

    // Show current configuration
    println!("\n{} {}", style("Current RPC URL:").cyan(), config.rpc_url);
    println!(
        "{} {:?}",
        style("Current Commitment Level:").cyan(),
        config.commitment_level
    );
    println!(
        "{} {}",
        style("Current Keypair Path:").cyan(),
        config.keypair_path.display()
    );

    // Prompt user to select which field to edit
    let field_options = ConfigField::all();
    let selected_field = Select::new("\nSelect field to edit:", field_options).prompt()?;

    match selected_field {
        ConfigField::RpcUrl => {
            config.rpc_url = prompt_data("Enter RPC URL:")?;
        }
        ConfigField::CommitmentLevel => {
            config.commitment_level =
                Select::new("Select commitment level:", get_commitment_levels()).prompt()?;
        }
        ConfigField::KeypairPath => {
            let default_keypair_path = ScillaConfig::default().keypair_path;

            loop {
                let keypair_input: PathBuf = prompt_data(&format!(
                    "Enter new keypair path (leave empty to use default: {}): ",
                    default_keypair_path.display()
                ))?;

                if keypair_input.as_os_str().is_empty() {
                    config.keypair_path = default_keypair_path.clone();
                    break;
                }

                if !keypair_input.exists() {
                    println!(
                        "{}",
                        style(format!(
                            "Keypair file not found at: {}",
                            keypair_input.display()
                        ))
                        .red()
                    );
                    continue;
                }

                config.keypair_path = keypair_input;
                break;
            }
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
