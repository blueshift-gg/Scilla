use {
    crate::{
        commands::CommandExec, constants::LAMPORTS_PER_SOL, context::ScillaContext,
        error::ScillaResult, ui::show_spinner,
    },
    comfy_table::{presets::UTF8_FULL, Cell, Table},
    console::style,
    std::{fmt, ops::Div},
};

/// Commands related to cluster operations
#[derive(Debug, Clone)]
pub enum ClusterCommand {
    EpochInfo,
    CurrentSlot,
    BlockHeight,
    BlockTime,
    Validators,
    SupplyInfo,
    Inflation,
    ClusterVersion,
    GoBack,
}

impl ClusterCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ClusterCommand::EpochInfo => "Fetching current epoch and progress…",
            ClusterCommand::CurrentSlot => "Fetching latest confirmed slot…",
            ClusterCommand::BlockHeight => "Fetching current block height…",
            ClusterCommand::BlockTime => "Fetching block timestamp…",
            ClusterCommand::Validators => "Fetching active validators…",
            ClusterCommand::ClusterVersion => "Fetching cluster Solana version…",
            ClusterCommand::SupplyInfo => "Fetching total and circulating supply…",
            ClusterCommand::Inflation => "Fetching inflation parameters…",
            ClusterCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for ClusterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            ClusterCommand::EpochInfo => "Epoch Info",
            ClusterCommand::CurrentSlot => "Current Slot",
            ClusterCommand::BlockHeight => "Block Height",
            ClusterCommand::BlockTime => "Block Time",
            ClusterCommand::Validators => "Validators",
            ClusterCommand::ClusterVersion => "Cluster Version",
            ClusterCommand::SupplyInfo => "Supply Info",
            ClusterCommand::Inflation => "Inflation",
            ClusterCommand::GoBack => "Go Back",
        };
        write!(f, "{command}")
    }
}

impl ClusterCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            ClusterCommand::EpochInfo => {
                show_spinner(self.spinner_msg(), fetch_epoch_info(ctx)).await?;
            }
            ClusterCommand::CurrentSlot => {
                show_spinner(self.spinner_msg(), fetch_current_slot(ctx)).await?;
            }
            ClusterCommand::BlockHeight => {
                show_spinner(self.spinner_msg(), fetch_block_height(ctx)).await?;
            }
            ClusterCommand::BlockTime => {
                show_spinner(self.spinner_msg(), fetch_block_time(ctx)).await?;
            }
            ClusterCommand::Validators => {
                show_spinner(self.spinner_msg(), fetch_validators(ctx)).await?;
            }
            ClusterCommand::SupplyInfo => {
                show_spinner(self.spinner_msg(), fetch_supply_info(ctx)).await?;
            }
            ClusterCommand::Inflation => {
                show_spinner(self.spinner_msg(), fetch_inflation_info(ctx)).await?;
            }
            ClusterCommand::ClusterVersion => {
                show_spinner(self.spinner_msg(), fetch_cluster_version(ctx)).await?;
            }
            ClusterCommand::GoBack => {
                return Ok(CommandExec::GoBack);
            }
        }

        Ok(CommandExec::Process(()))
    }
}

async fn fetch_epoch_info(ctx: &ScillaContext) -> anyhow::Result<()> {
    let epoch_info = ctx.rpc().get_epoch_info().await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Epoch"),
            Cell::new(format!("{}", epoch_info.epoch)),
        ])
        .add_row(vec![
            Cell::new("Slot Index"),
            Cell::new(format!("{}", epoch_info.slot_index)),
        ])
        .add_row(vec![
            Cell::new("Slots in Epoch"),
            Cell::new(format!("{}", epoch_info.slots_in_epoch)),
        ])
        .add_row(vec![
            Cell::new("Absolute Slot"),
            Cell::new(format!("{}", epoch_info.absolute_slot)),
        ])
        .add_row(vec![
            Cell::new("Block Height"),
            Cell::new(format!("{}", epoch_info.block_height)),
        ])
        .add_row(vec![
            Cell::new("Transaction Count"),
            Cell::new(format!("{}", epoch_info.transaction_count.unwrap_or(0))),
        ]);

    println!("\n{}", style("EPOCH INFORMATION").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_current_slot(ctx: &ScillaContext) -> anyhow::Result<()> {
    let slot = ctx.rpc().get_slot().await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Current Slot"),
            Cell::new(format!("{slot}")),
        ]);

    println!("\n{}", style("CURRENT SLOT").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_block_height(ctx: &ScillaContext) -> anyhow::Result<()> {
    let block_height = ctx.rpc().get_block_height().await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Block Height"),
            Cell::new(format!("{block_height}")),
        ]);

    println!("\n{}", style("BLOCK HEIGHT").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_block_time(ctx: &ScillaContext) -> anyhow::Result<()> {
    let slot = ctx.rpc().get_slot().await?;
    let block_time = ctx.rpc().get_block_time(slot).await?;

    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp_secs(block_time)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid timestamp".to_string());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Slot"), Cell::new(format!("{slot}"))])
        .add_row(vec![
            Cell::new("Unix Timestamp"),
            Cell::new(format!("{block_time}")),
        ])
        .add_row(vec![Cell::new("Date/Time"), Cell::new(datetime)]);

    println!("\n{}", style("BLOCK TIME").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_validators(ctx: &ScillaContext) -> anyhow::Result<()> {
    let validators = ctx.rpc().get_vote_accounts().await?;

    // Summary table
    let mut summary_table = Table::new();
    summary_table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Current Validators"),
            Cell::new(format!("{}", validators.current.len())),
        ])
        .add_row(vec![
            Cell::new("Delinquent Validators"),
            Cell::new(format!("{}", validators.delinquent.len())),
        ]);

    println!("\n{}", style("VALIDATORS SUMMARY").green().bold());
    println!("{summary_table}");

    // Validators detail table
    if !validators.current.is_empty() {
        let mut validators_table = Table::new();
        validators_table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("#").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Node Pubkey").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Vote Account").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Activated Stake (SOL)").add_attribute(comfy_table::Attribute::Bold),
        ]);

        for (idx, validator) in validators.current.iter().enumerate() {
            let stake_sol = (validator.activated_stake as f64).div(LAMPORTS_PER_SOL as f64);
            validators_table.add_row(vec![
                Cell::new(format!("{}", idx + 1)),
                Cell::new(validator.node_pubkey.clone()),
                Cell::new(validator.vote_pubkey.clone()),
                Cell::new(format!("{stake_sol:.2}")),
            ]);
        }

        println!("\n{}", style("TOP VALIDATORS").green().bold());
        println!("{validators_table}");
    }

    Ok(())
}

async fn fetch_supply_info(ctx: &ScillaContext) -> anyhow::Result<()> {
    let supply = ctx.rpc().supply().await?;

    let total_sol = (supply.value.total as f64).div(LAMPORTS_PER_SOL as f64);
    let circulating_sol = (supply.value.circulating as f64).div(LAMPORTS_PER_SOL as f64);
    let non_circulating_sol = (supply.value.non_circulating as f64).div(LAMPORTS_PER_SOL as f64);
    let circulating_pct = (circulating_sol / total_sol) * 100.0;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value (SOL)").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Percentage").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Total Supply"),
            Cell::new(format!("{total_sol:.2}")),
            Cell::new("100.00%"),
        ])
        .add_row(vec![
            Cell::new("Circulating"),
            Cell::new(format!("{circulating_sol:.2}")),
            Cell::new(format!("{circulating_pct:.2}%")),
        ])
        .add_row(vec![
            Cell::new("Non-Circulating"),
            Cell::new(format!("{non_circulating_sol:.2}")),
            Cell::new(format!("{:.2}%", 100.0 - circulating_pct)),
        ]);

    println!("\n{}", style("SUPPLY INFORMATION").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_inflation_info(ctx: &ScillaContext) -> anyhow::Result<()> {
    let inflation = ctx.rpc().get_inflation_rate().await?;
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Epoch"),
            Cell::new(format!("{}", inflation.epoch)),
        ])
        .add_row(vec![
            Cell::new("Total Inflation Rate"),
            Cell::new(format!("{:.4}%", inflation.total * 100.0)),
        ])
        .add_row(vec![
            Cell::new("Validator Inflation"),
            Cell::new(format!("{:.4}%", inflation.validator * 100.0)),
        ])
        .add_row(vec![
            Cell::new("Foundation Inflation"),
            Cell::new(format!("{:.4}%", inflation.foundation * 100.0)),
        ]);

    println!("\n{}", style("INFLATION INFORMATION").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_cluster_version(ctx: &ScillaContext) -> anyhow::Result<()> {
    let version = ctx.rpc().get_version().await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Solana Core"),
            Cell::new(version.solana_core.clone()),
        ]);

    if let Some(feature_set) = version.feature_set {
        table.add_row(vec![
            Cell::new("Feature Set"),
            Cell::new(format!("{feature_set}")),
        ]);
    }

    println!("\n{}", style("CLUSTER VERSION").green().bold());
    println!("{table}");

    Ok(())
}
