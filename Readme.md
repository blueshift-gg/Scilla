# **Scilla — An Interactive Solana CLI**

**Scilla** is a fast, developer-friendly interactive command-line interface for Solana.
Instead of manually typing long CLI flags, Scilla uses intelligent prompting to help you build and execute commands seamlessly — making the workflow far less tedious.

With Scilla, you can easily **inspect on-chain data**, **query cluster state**, **send transactions**, and perform a wide range of Solana development and debugging tasks — all from a unified interactive shell.

---

## **Quick Start**

### **1. Install**

```bash
# Clone and build
git clone https://github.com/blueshift-gg/Scilla
cd Scilla
cargo install --path .
```
---

## **Usage**

Launch Scilla and you'll see:

```bash 
scilla
```

``` bash
⚡ Scilla — Hacking Through the Solana Matrix

? Choose a command group:
  > Account
    Cluster
    Stake
    Vote
    ScillaConfig
    Exit
```

Navigate using arrow keys, press Enter to select.

### **2. Run & Configure**

```bash
# Go to ScillaConfig > Generate ScillaConfig 

⚡ Scilla — Hacking Through the Solana Matrix
> Choose a command group: ScillaConfig
? ScillaConfig Command:
  Show ScillaConfig
> Generate ScillaConfig
  Edit ScillaConfig

```

This will generate a config file with your desired parameters. For example:

```toml
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "~/.config/solana/id.json"
commitment-level = "confirmed"
```

You can then edit the generated `~/.config/scilla.toml` going to ScillaConfig > Edit ScillaConfig, or manually editing the file.


---

## **Commands**

### **Account**

Manage wallets and on-chain accounts.

| Command                 | What it does                         | Status |
| ----------------------- | ------------------------------------ | ------ |
| **Fetch Account**       | Fetch Account                        | Done   |
| **Balance**             | Check SOL balance                    | Done   |
| **Transfer**            | Send SOL to another wallet           | Todo   |
| **Airdrop**             | Request devnet/testnet SOL           | Done   |
| **Check Transaction Confirmation** | Check if a transaction landed        | Done   |
| **Largest Accounts**    | See the biggest accounts on cluster  | Done   |
| **Nonce Account**       | Inspect or manage durable nonces     | Done   |

**Example flow:**

```
? Choose a command group: Account
? Account Command: Balance
? Enter Pubkey: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
⠴ Get Account Balance
Account balance in SOL:
Balance: 1.5 SOL
```

---

### **Cluster**

Query the state of the Solana cluster.

| Command             | What it does                      | Status |
| ------------------- | --------------------------------- | ------ |
| **Epoch Info**      | Current epoch and progress        | Done   |
| **Current Slot**    | Latest confirmed slot             | Done   |
| **Block Height**    | Current block height              | Done   |
| **Block Time**      | Timestamp for a specific block    | Done   |
| **Validators**      | List active validators            | Done   |
| **Cluster Version** | Solana version running on cluster | Done   |
| **Supply Info**     | Total and circulating supply      | Done   |
| **Inflation**       | Current inflation parameters      | Done   |

---

### **Stake**

Full stake account lifecycle management.

| Command        | What it does                        | Status |
| -------------- | ----------------------------------- | ------ |
| **Create**     | Create a new stake account          | Done   |
| **Delegate**   | Delegate stake to a validator       | Done   |
| **Deactivate** | Begin stake cooldown                | Done   |
| **Withdraw**   | Withdraw SOL from deactivated stake | Done   |
| **Merge**      | Combine two stake accounts          | Todo   |
| **Split**      | Split stake into multiple accounts  | Todo   |
| **Show**       | Display stake account details       | Todo   |
| **History**    | View stake account history          | Todo   |

---

### **Vote**

For validators managing vote accounts.

| Command                 | What it does                  | Status |
| ----------------------- | ----------------------------- | ------ |
| **Create Vote Account** | Initialize a new vote account | Done   |
| **Authorize Voter**     | Change authorized voter       | Done   |
| **Withdraw from Vote**  | Withdraw from vote account    | Done   |
| **Show Vote Account**   | Display vote account info     | Done   |

---

## **ScillaConfig**

Manage Scilla's configuration settings.

| Command                   | What it does                       | Status |
| ------------------------- | ---------------------------------- | ------ |
| **Generate ScillaConfig** | Create or overwrite config file    | Done   |
| **Edit ScillaConfig**     | Open config file in default editor | Done   |
| **Show ScillaConfig**     | Display current config settings    | Done   |
