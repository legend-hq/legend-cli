mod auth;
mod commands;
mod config;
mod output;

use clap::{Parser, Subcommand};
use legend_client::*;

use crate::auth::{resolve_base_url, resolve_query_key};
use crate::config::Env;
use crate::output::*;

#[derive(Parser)]
#[command(name = "legend-cli", about = "Legend API client & signing tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Use a specific key profile
    #[arg(long, default_value = "default", global = true)]
    profile: String,

    /// Override query key
    #[arg(long, global = true)]
    key: Option<String>,

    /// Override API base URL (overrides --env)
    #[arg(long, global = true)]
    base_url: Option<String>,

    /// Target environment
    #[arg(long, value_enum, default_value = "prod", global = true)]
    env: Env,

    /// Shorthand for --env dev
    #[arg(long, global = true, conflicts_with_all = ["stage", "prod_flag"])]
    dev: bool,

    /// Shorthand for --env stage
    #[arg(long, global = true, conflicts_with_all = ["dev", "prod_flag"])]
    stage: bool,

    /// Shorthand for --env prod
    #[arg(long = "prod", global = true, conflicts_with_all = ["dev", "stage"], id = "prod_flag")]
    prod_flag: bool,

    /// Force JSON output
    #[arg(long, global = true)]
    json: bool,

    /// Minimal output (IDs only)
    #[arg(long, global = true)]
    quiet: bool,

    /// Log HTTP requests and responses to stderr
    #[arg(long, short = 'v', global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage sub-accounts
    Accounts {
        #[command(subcommand)]
        action: AccountsAction,
    },
    /// View account portfolio
    Folio {
        account_id: String,
        #[arg(long)]
        cached: bool,
    },
    /// Create and execute plans
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },
    /// View account activities
    Activities {
        account_id: String,
        #[arg(long)]
        id: Option<u64>,
    },
    /// Sign a digest with the active profile's P256 key via Turnkey
    Sign { digest: String },
    /// List Turnkey wallets for the active profile's sub-org
    Wallets,
    /// Generate a P256 keypair without creating an account
    Keygen {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        use_file_key: bool,
    },
    /// Manage local Keychain keys
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },
    /// Log in via Google SSO (OAuth 2.1 + PKCE)
    Login,
    /// Run as an MCP server (stdio transport)
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// List available networks
    Networks,
    /// List available assets
    Assets,
    /// Show current auth info
    Whoami,
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum AccountsAction {
    /// List all sub-accounts
    List,
    /// Get a sub-account by ID
    Get { account_id: String },
    /// Create a new sub-account
    Create {
        /// Signer type (not needed with --keygen)
        #[arg(long, value_enum, required_unless_present = "keygen")]
        signer_type: Option<SignerType>,
        /// Ethereum signer address (for eoa accounts)
        #[arg(long)]
        ethereum_signer: Option<String>,
        /// Solana signer address (for eoa accounts)
        #[arg(long)]
        solana_signer: Option<String>,
        /// P256 public key (for turnkey_p256 without --keygen)
        #[arg(long)]
        p256_public_key: Option<String>,
        /// Generate a new P256 key and create a turnkey_p256 account
        #[arg(long)]
        keygen: bool,
        /// Profile name for the generated key
        #[arg(long)]
        name: Option<String>,
        /// Use file-based key instead of Secure Enclave
        #[arg(long)]
        use_file_key: bool,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum SignerType {
    Eoa,
    TurnkeyP256,
}

impl SignerType {
    fn as_api_str(&self) -> &'static str {
        match self {
            SignerType::Eoa => "eoa",
            SignerType::TurnkeyP256 => "turnkey_p256",
        }
    }
}

#[derive(Subcommand)]
enum PlanAction {
    /// Create an earn plan
    Earn {
        account_id: String,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        network: String,
        #[arg(long)]
        protocol: String,
        #[arg(long)]
        market: Option<String>,
        /// Create plan, sign, and execute in one step
        #[arg(long)]
        execute: bool,
    },
    /// Create a swap plan
    Swap {
        account_id: String,
        #[arg(long)]
        sell_asset: String,
        #[arg(long)]
        buy_asset: String,
        #[arg(long)]
        sell_amount: Option<String>,
        #[arg(long)]
        buy_amount: Option<String>,
        #[arg(long)]
        network: String,
        #[arg(long)]
        execute: bool,
    },
    /// Create a borrow plan
    Borrow {
        account_id: String,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        collateral_amount: String,
        #[arg(long)]
        collateral_asset: String,
        #[arg(long)]
        network: String,
        #[arg(long)]
        protocol: String,
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        execute: bool,
    },
    /// Create a withdraw plan
    Withdraw {
        account_id: String,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        network: String,
        #[arg(long)]
        protocol: String,
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        execute: bool,
    },
    /// Create a repay plan
    Repay {
        account_id: String,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        collateral_amount: String,
        #[arg(long)]
        collateral_asset: String,
        #[arg(long)]
        network: String,
        #[arg(long)]
        protocol: String,
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        execute: bool,
    },
    /// Create a transfer plan
    Transfer {
        account_id: String,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        network: String,
        #[arg(long)]
        recipient: String,
        #[arg(long)]
        execute: bool,
    },
    /// Execute a plan with a signature
    Execute {
        account_id: String,
        #[arg(long)]
        plan_id: String,
        /// Sign automatically using the profile's P256 key via Turnkey
        #[arg(long)]
        auto_sign: bool,
        /// Provide the plan digest for auto-signing
        #[arg(long)]
        digest: Option<String>,
        /// Provide a pre-computed signature
        #[arg(long)]
        signature: Option<String>,
    },
}

#[derive(Subcommand)]
enum KeysAction {
    /// Create a new key
    Create {
        /// Key name
        name: String,
        /// Use file-based key instead of the default
        #[arg(long, conflicts_with = "keychain")]
        file: bool,
        /// Use iCloud Keychain (requires brew install)
        #[arg(long, conflicts_with = "file")]
        keychain: bool,
    },
    /// List all keys for the current environment
    List,
    /// Sign a hex digest with a local key (no Turnkey)
    Sign {
        /// Key name
        name: String,
        /// Hex-encoded digest to sign
        digest: String,
    },
    /// Delete a key
    Delete {
        /// Key name
        name: String,
        /// Delete a file-based key instead of the default
        #[arg(long, conflicts_with = "keychain")]
        file: bool,
        /// Delete an iCloud Keychain key (requires brew install)
        #[arg(long, conflicts_with = "file")]
        keychain: bool,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Start the MCP stdio server
    Serve,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a configuration value (e.g. query-key)
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mode = detect_mode(cli.json, cli.quiet);
    let env = if cli.dev {
        Env::Dev
    } else if cli.stage {
        Env::Stage
    } else if cli.prod_flag {
        Env::Prod
    } else {
        cli.env
    };
    let base_url = resolve_base_url(&cli.base_url, env);
    let base = Some(base_url.clone());

    let result = match &cli.command {
        Commands::Accounts { action } => match action {
            AccountsAction::List => {
                commands::accounts::list(&cli.key, env, &cli.profile, &base, cli.verbose, &mode)
                    .await
            }
            AccountsAction::Get { account_id } => {
                commands::accounts::get(
                    account_id,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            AccountsAction::Create {
                signer_type,
                ethereum_signer,
                solana_signer,
                p256_public_key,
                keygen,
                name,
                use_file_key,
            } => {
                let effective_signer_type = if *keygen {
                    "turnkey_p256"
                } else {
                    signer_type.as_ref().unwrap().as_api_str()
                };
                commands::accounts::create(
                    effective_signer_type,
                    ethereum_signer,
                    solana_signer,
                    p256_public_key,
                    *keygen,
                    name,
                    *use_file_key,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
        },

        Commands::Folio { account_id, cached } => {
            async {
                let client = make_client(&cli.key, env, &cli.profile, &base, cli.verbose)?;
                let folio = client
                    .accounts
                    .folio(account_id, &FolioOpts { cached: *cached })
                    .await?;
                print_folio(&folio, &mode);
                Ok(())
            }
            .await
        }

        Commands::Plan { action } => match action {
            PlanAction::Earn {
                account_id,
                amount,
                asset,
                network,
                protocol,
                market,
                execute,
            } => {
                commands::plan::earn(
                    account_id,
                    amount,
                    asset,
                    network,
                    protocol,
                    market,
                    *execute,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            PlanAction::Swap {
                account_id,
                sell_asset,
                buy_asset,
                sell_amount,
                buy_amount,
                network,
                execute,
            } => {
                commands::plan::swap(
                    account_id,
                    sell_asset,
                    buy_asset,
                    sell_amount,
                    buy_amount,
                    network,
                    *execute,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            PlanAction::Borrow {
                account_id,
                amount,
                asset,
                collateral_amount,
                collateral_asset,
                network,
                protocol,
                market,
                execute,
            } => {
                commands::plan::borrow(
                    account_id,
                    amount,
                    asset,
                    collateral_amount,
                    collateral_asset,
                    network,
                    protocol,
                    market,
                    *execute,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            PlanAction::Withdraw {
                account_id,
                amount,
                asset,
                network,
                protocol,
                market,
                execute,
            } => {
                commands::plan::withdraw(
                    account_id,
                    amount,
                    asset,
                    network,
                    protocol,
                    market,
                    *execute,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            PlanAction::Repay {
                account_id,
                amount,
                asset,
                collateral_amount,
                collateral_asset,
                network,
                protocol,
                market,
                execute,
            } => {
                commands::plan::repay(
                    account_id,
                    amount,
                    asset,
                    collateral_amount,
                    collateral_asset,
                    network,
                    protocol,
                    market,
                    *execute,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            PlanAction::Transfer {
                account_id,
                amount,
                asset,
                network,
                recipient,
                execute,
            } => {
                commands::plan::transfer(
                    account_id,
                    amount,
                    asset,
                    network,
                    recipient,
                    *execute,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
            PlanAction::Execute {
                account_id,
                plan_id,
                auto_sign,
                digest,
                signature,
            } => {
                commands::plan::execute_plan(
                    account_id,
                    plan_id,
                    *auto_sign,
                    digest,
                    signature,
                    &cli.key,
                    env,
                    &cli.profile,
                    &base,
                    cli.verbose,
                    &mode,
                )
                .await
            }
        },

        Commands::Activities { account_id, id } => {
            async {
                let client = make_client(&cli.key, env, &cli.profile, &base, cli.verbose)?;
                if let Some(activity_id) = id {
                    let activity = client.accounts.activity(account_id, *activity_id).await?;
                    print_activity(&activity, &mode);
                } else {
                    let list = client.accounts.activities(account_id).await?;
                    print_activities(&list, &mode);
                }
                Ok(())
            }
            .await
        }

        Commands::Sign { digest } => {
            commands::sign::sign(digest, env, &cli.profile, cli.verbose).await
        }

        Commands::Wallets => {
            async {
                let profile = config::load_profile(env, &cli.profile)
                    .ok_or_else(|| anyhow::anyhow!("No profile found. Run: legend-cli login"))?;
                let signer = commands::sign::load_signer_from_profile(&profile)?;
                let turnkey = legend_signer::TurnkeyClient::new(legend_signer::TurnkeyConfig {
                    signer,
                    sub_org_id: profile.sub_org_id,
                    ethereum_signer_address: profile.ethereum_signer_address,
                    api_base_url: None,
                    verbose: cli.verbose,
                });
                let wallets = turnkey.list_wallets().await?;
                if let Some(wallet_list) = wallets["wallets"].as_array() {
                    for wallet in wallet_list {
                        let wallet_id = wallet["walletId"].as_str().unwrap_or("?");
                        eprintln!("Wallet: {} ({})", wallet["walletName"], wallet_id);
                        let accounts = turnkey.list_wallet_accounts(wallet_id).await?;
                        println!("{}", serde_json::to_string_pretty(&accounts)?);
                    }
                }
                Ok(())
            }
            .await
        }

        Commands::Login => commands::login::login(&base_url, env, &cli.profile).await,

        Commands::Mcp { action } => match action {
            McpAction::Serve => commands::mcp::serve(env, &cli.key, &cli.profile).await,
        },

        Commands::Keygen { name, use_file_key } => {
            run_keygen(name.as_deref(), *use_file_key, env, &cli.profile)
        }

        Commands::Keys { action } => match action {
            KeysAction::Create {
                name,
                file,
                keychain,
            } => commands::keys::create(name, env, *file, *keychain),
            KeysAction::List => commands::keys::list(env, cli.verbose),
            KeysAction::Sign { name, digest } => commands::keys::sign(name, digest, env),
            KeysAction::Delete {
                name,
                file,
                keychain,
            } => commands::keys::delete(name, env, *file, *keychain),
        },

        Commands::Networks => {
            async {
                let client = make_client(&cli.key, env, &cli.profile, &base, cli.verbose)?;
                let networks = client.networks().await?;
                print_networks(&networks, &mode);
                Ok(())
            }
            .await
        }
        Commands::Assets => {
            async {
                let client = make_client(&cli.key, env, &cli.profile, &base, cli.verbose)?;
                let assets = client.assets().await?;
                print_assets(&assets, &mode);
                Ok(())
            }
            .await
        }

        Commands::Whoami => {
            async {
                let client = make_client(&cli.key, env, &cli.profile, &base, cli.verbose)?;
                let pa = client.prime_account().await?;
                print_prime_account(&pa, &mode);
                Ok(())
            }
            .await
        }

        Commands::Config { action } => match action {
            ConfigAction::Set { key, value } => run_config_set(key, value, env, &cli.profile),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// --- Helpers ---

fn make_client(
    key: &Option<String>,
    env: Env,
    profile: &str,
    base_url: &Option<String>,
    verbose: bool,
) -> anyhow::Result<LegendPrime> {
    let query_key = resolve_query_key(key, env, profile).map_err(anyhow::Error::msg)?;
    Ok(LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    }))
}

fn run_keygen(
    name: Option<&str>,
    use_file_key: bool,
    env: Env,
    profile: &str,
) -> anyhow::Result<()> {
    use legend_signer::*;

    let effective_name = name.unwrap_or(profile);

    if use_file_key {
        let dir = config::keys_dir(env);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{effective_name}.key"));
        let signer = FileSigner::generate(&path)?;
        eprintln!("Key saved to {}", path.display());
        println!("{}", signer.public_key_hex());
    } else {
        #[cfg(feature = "keychain")]
        {
            let label = format!("com.legend.cli.{env}.{effective_name}");
            let signer = legend_signer::KeychainSigner::generate(&label)?;
            eprintln!("Key saved to iCloud Keychain (label: {label})");
            println!("{}", signer.public_key_hex());
        }
        #[cfg(not(feature = "keychain"))]
        {
            anyhow::bail!(
                "iCloud Keychain is not available in this build. Use --use-file-key,\n\
                 or install via `brew install legend-cli` for iCloud Keychain support."
            );
        }
    }
    Ok(())
}

fn run_config_set(key: &str, value: &str, env: Env, profile_name: &str) -> anyhow::Result<()> {
    match key {
        "query-key" => {
            let mut p =
                config::load_profile(env, profile_name).unwrap_or_else(|| config::Profile {
                    query_key: None,
                    key_source: "none".into(),
                    key_label: None,
                    key_path: None,
                    p256_public_key: String::new(),
                    sub_org_id: String::new(),
                    ethereum_signer_address: String::new(),
                    account_external_id: String::new(),
                });
            p.query_key = Some(value.to_string());
            config::save_profile(env, profile_name, &p)?;
            eprintln!("Query key saved to profile '{profile_name}' ({env})");
            Ok(())
        }
        _ => anyhow::bail!("Unknown config key: {key}. Available: query-key"),
    }
}
