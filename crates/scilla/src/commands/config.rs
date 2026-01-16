use {
    crate::{
        commands::CommandFlow,
        config::{ScillaConfig, scilla_config_path},
        context::ScillaContext,
        misc::helpers::short_pubkey,
        prompt::{prompt_input_data, prompt_keypair_path, prompt_network_rpc_url},
        ui::print_error,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::{Confirm, Select},
    serde::{Deserialize, Serialize},
    solana_commitment_config::CommitmentLevel,
    std::{fmt, fs, path::PathBuf},
};

/// Commands related to configuration like RPC_URL , KEYAPAIR_PATH etc
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Edit,
    GoBack,
}

impl ConfigCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ConfigCommand::Show => "Displaying current Scilla configuration…",
            ConfigCommand::Edit => "Editing existing Scilla configuration…",
            ConfigCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for ConfigCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            ConfigCommand::Show => "View ScillaConfig",
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
    None,
}

impl fmt::Display for ConfigField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigField::RpcUrl => write!(f, "RPC URL"),
            ConfigField::CommitmentLevel => write!(f, "Commitment Level"),
            ConfigField::KeypairPath => write!(f, "Keypair Path"),
            ConfigField::None => write!(f, "None"),
        }
    }
}

impl ConfigField {
    fn all() -> Vec<Self> {
        vec![
            ConfigField::RpcUrl,
            ConfigField::CommitmentLevel,
            ConfigField::KeypairPath,
            ConfigField::None,
        ]
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UICommitmentOptions {
    Level(CommitmentLevel),
    None,
}

impl fmt::Display for UICommitmentOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UICommitmentOptions::Level(level) => write!(f, "{level:?}"),
            UICommitmentOptions::None => write!(f, "None"),
        }
    }
}

fn get_commitment_levels() -> Vec<UICommitmentOptions> {
    vec![
        UICommitmentOptions::Level(CommitmentLevel::Processed),
        UICommitmentOptions::Level(CommitmentLevel::Confirmed),
        UICommitmentOptions::Level(CommitmentLevel::Finalized),
        UICommitmentOptions::None,
    ]
}

impl ConfigCommand {
    pub fn process_command(&self, ctx: &mut ScillaContext) -> CommandFlow<()> {
        let res = match self {
            ConfigCommand::Show => show_config(ctx),
            ConfigCommand::Edit => edit_config(ctx),
            ConfigCommand::GoBack => return CommandFlow::GoBack,
        };

        if let Err(e) = res {
            print_error(e.to_string())
        }

        CommandFlow::Process(())
    }
}

fn show_config(ctx: &ScillaContext) -> anyhow::Result<()> {
    let mut table = Table::new();
    let config = ScillaConfig::load()?;

    let wallet_pubkey = ctx.pubkey();
    let keypair_display = format!(
        "{} ({})",
        config.keypair_path.display(),
        short_pubkey(wallet_pubkey),
    );
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
            Cell::new("Value")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
        ])
        .add_row(vec![Cell::new("RPC URL"), Cell::new(config.rpc_url)])
        .add_row(vec![
            Cell::new("Commitment Level"),
            Cell::new(config.commitment_level),
        ])
        .add_row(vec![Cell::new("Keypair Path"), Cell::new(keypair_display)]);

    println!("\n{}", style("SCILLA CONFIG").green().bold());
    println!("{table}");

    Ok(())
}

pub fn generate_config() -> anyhow::Result<()> {
    // Check if config already exists
    let config_path = scilla_config_path();
    if config_path.exists() {
        println!("{}", style("Config file already exists!").yellow().bold());
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

        println!("{}", style("Using default configuration:").cyan());
        println!("  RPC: {}", config.rpc_url);
        println!("  Commitment: {:?}", config.commitment_level);
        println!("  Keypair: {}", config.keypair_path.display());

        config
    } else {
        let rpc_url: String = prompt_input_data("Enter RPC URL:");

        let commitment_level =
            match Select::new("Select commitment level:", get_commitment_levels()).prompt()? {
                UICommitmentOptions::Level(level) => level,
                UICommitmentOptions::None => return Ok(()),
            };

        let keypair_path = loop {
            let keypair_input: PathBuf = prompt_input_data("Enter keypair path:");

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

    println!("{}", style("Config generated successfully!").green().bold());
    println!(
        "{}",
        style(format!("Saved to: {}", config_path.display())).cyan()
    );

    Ok(())
}

fn edit_config(ctx: &mut ScillaContext) -> anyhow::Result<()> {
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
    let selected_field = Select::new("Select field to edit:", field_options).prompt()?;

    match selected_field {
        ConfigField::RpcUrl => {
            let choice = Select::new(
                "Do you want to use a custom RPC Url or one of the defaults?",
                vec!["Default", "Custom"],
            )
            .prompt()?;
            let new_rpc_url = match choice {
                "Default" => prompt_network_rpc_url()?,
                "Custom" => prompt_input_data::<String>("Enter custom RPC URL:"),
                _ => unreachable!(),
            };

            config.rpc_url = new_rpc_url;
        }
        ConfigField::CommitmentLevel => {
            let selected =
                Select::new("Select Commitment Level", get_commitment_levels()).prompt()?;

            let level = match selected {
                UICommitmentOptions::Level(level) => level,
                UICommitmentOptions::None => return Ok(()),
            };

            config.commitment_level = level
        }
        ConfigField::KeypairPath => loop {
            let keypair_input = prompt_keypair_path("Enter new keypair path:", ctx);

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
        },
        ConfigField::None => return Ok(()),
    }

    // Write updated config
    let config_path = scilla_config_path();
    let toml_string = toml::to_string_pretty(&config)?;
    fs::write(&config_path, toml_string)?;

    ctx.reload(config)?;

    println!("{}", style("Config updated successfully!").green().bold());
    println!(
        "{}",
        style(format!("Saved to: {}", config_path.display())).cyan()
    );

    Ok(())
}
