/// Tests for command description methods
/// These are simple tests but ensure documentation stays in sync with code
use scilla::commands::account::AccountCommand;
use scilla::commands::cluster::ClusterCommand;
use scilla::commands::stake::StakeCommand;
use scilla::commands::vote::VoteCommand;

// ============================================================================
// Account Command Descriptions
// ============================================================================

#[test]
fn test_account_command_descriptions_not_empty() {
    let commands = vec![
        AccountCommand::FetchAccount,
        AccountCommand::Balance,
        AccountCommand::Transfer,
        AccountCommand::Airdrop,
        AccountCommand::ConfirmTransaction,
        AccountCommand::LargestAccounts,
        AccountCommand::NonceAccount,
        AccountCommand::GoBack,
    ];

    for cmd in commands {
        let desc = cmd.description();
        assert!(
            !desc.is_empty(),
            "Description for {:?} should not be empty",
            cmd
        );
    }
}

#[test]
fn test_account_command_specific_descriptions() {
    assert_eq!(AccountCommand::FetchAccount.description(), "Fetch Account");
    assert_eq!(AccountCommand::Balance.description(), "Check SOL balance");
    assert_eq!(
        AccountCommand::Transfer.description(),
        "Send SOL to another wallet"
    );
    assert_eq!(
        AccountCommand::Airdrop.description(),
        "Request devnet/testnet SOL"
    );
}

// ============================================================================
// Cluster Command Descriptions
// ============================================================================

#[test]
fn test_cluster_command_descriptions_not_empty() {
    let commands = vec![
        ClusterCommand::EpochInfo,
        ClusterCommand::CurrentSlot,
        ClusterCommand::BlockHeight,
        ClusterCommand::BlockTime,
        ClusterCommand::Validators,
        ClusterCommand::SupplyInfo,
        ClusterCommand::Inflation,
        ClusterCommand::ClusterVersion,
        ClusterCommand::GoBack,
    ];

    for cmd in commands {
        let desc = cmd.description();
        assert!(
            !desc.is_empty(),
            "Description for {:?} should not be empty",
            cmd
        );
    }
}

#[test]
fn test_cluster_command_specific_descriptions() {
    assert_eq!(
        ClusterCommand::EpochInfo.description(),
        "Current epoch and progress"
    );
    assert_eq!(
        ClusterCommand::CurrentSlot.description(),
        "Latest confirmed slot"
    );
    assert_eq!(
        ClusterCommand::BlockHeight.description(),
        "Current block height"
    );
}

// ============================================================================
// Stake Command Descriptions
// ============================================================================

#[test]
fn test_stake_command_descriptions_not_empty() {
    let commands = vec![
        StakeCommand::Create,
        StakeCommand::Delegate,
        StakeCommand::Deactivate,
        StakeCommand::Withdraw,
        StakeCommand::Merge,
        StakeCommand::Split,
        StakeCommand::Show,
        StakeCommand::History,
        StakeCommand::GoBack,
    ];

    for cmd in commands {
        let desc = cmd.description();
        assert!(
            !desc.is_empty(),
            "Description for {:?} should not be empty",
            cmd
        );
    }
}

// ============================================================================
// Vote Command Tests (No description method, but we can test enum variants)
// ============================================================================

#[test]
fn test_vote_command_variants_exist() {
    // Just ensure all variants can be constructed
    let _commands = [
        VoteCommand::CreateVoteAccount,
        VoteCommand::AuthorizeVoter,
        VoteCommand::WithdrawFromVoteAccount,
        VoteCommand::ShowVoteAccount,
        VoteCommand::GoBack,
    ];
    // If this compiles, all variants exist
}
