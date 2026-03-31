//! MCP stdio server — exposes CLI capabilities as MCP tools.
//!
//! Run with: `legend-cli mcp serve`
//!
//! Reads JSON-RPC from stdin, writes responses to stdout.
//! Provides the full Legend workflow as tools: accounts, portfolio, and
//! action tools (earn, withdraw, swap, transfer, borrow, repay, migrate,
//! swap_and_supply, claim_rewards, reinvest_rewards, loop_long, unloop_long,
//! add_backing, withdraw_backing) that create plans, sign via the local P256
//! key, execute, and optionally wait for completion — all in a single tool call.

use std::io::{self, BufRead, Write};

use legend_client::*;
use legend_signer::*;
use serde_json::{Value, json};

use crate::auth::{resolve_base_url, resolve_query_key};
use crate::commands::sign::load_signer_from_profile;
use crate::config::{self, Env};

const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "legend-cli";
const SERVER_VERSION: &str = "0.0.1";

/// uint256 max — used as the sentinel for "withdraw/earn all available".
const UINT256_MAX: &str = "115792089237316195423570985008687907853269984665640564039457584007913129639935";

struct McpSession {
    env: Env,
    key: Option<String>,
    profile: String,
    active_account_id: Option<String>,
}

pub async fn serve(env: Env, key: &Option<String>, profile: &str) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Load persisted account from profile if available.
    let persisted_account = config::load_profile(env, profile)
        .and_then(|p| {
            if p.account_external_id.is_empty() {
                None
            } else {
                Some(p.account_external_id)
            }
        });

    let mut session = McpSession {
        env,
        key: key.clone(),
        profile: profile.to_string(),
        active_account_id: persisted_account,
    };

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");

        let response = match method {
            "initialize" => Some(jsonrpc_result(
                id,
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
                }),
            )),

            "notifications/initialized" => None,

            "tools/list" => Some(jsonrpc_result(id, json!({ "tools": tool_definitions() }))),

            "tools/call" => {
                let name = msg["params"]["name"].as_str().unwrap_or("");
                let args = msg["params"]["arguments"].clone();
                let result = handle_tool_call(name, args, &mut session).await;
                Some(match result {
                    Ok(text) => {
                        jsonrpc_result(id, json!({ "content": [{ "type": "text", "text": text }] }))
                    }
                    Err(e) => jsonrpc_result(
                        id,
                        json!({ "content": [{ "type": "text", "text": format!("Error: {e}") }], "isError": true }),
                    ),
                })
            }

            _ if id.is_some() => Some(jsonrpc_error(id, -32601, "Method not found")),
            _ => None,
        };

        if let Some(resp) = response {
            let out = serde_json::to_string(&resp)?;
            writeln!(stdout, "{out}")?;
            stdout.flush()?;
        }
    }

    Ok(())
}

// --- Tool definitions ---

fn tool_definitions() -> Vec<Value> {
    vec![
        // --- Auth & account management ---
        tool_def(
            "login",
            "Log in via Google SSO. Opens a browser for authentication and saves the token to the active profile.",
            json!({}),
            vec![],
        ),
        tool_def(
            "whoami",
            "Show current authentication info — which Prime Account is active. Only needed for debugging or verification; most tools work without calling this first.",
            json!({}),
            vec![],
        ),
        tool_def(
            "set_account",
            "Set the active account for this session and persist it to disk. Once set, account_id becomes optional on all other tools. The active account is remembered across sessions — you usually do not need to call this unless switching accounts. Call list_accounts to see available accounts.",
            json!({
                "account_id": { "type": "string", "description": "Account ID (e.g. \"acc_xxx\")" }
            }),
            vec!["account_id"],
        ),
        tool_def(
            "list_accounts",
            "List all sub-accounts under the authenticated Prime Account. Use set_account with one of the returned account IDs to avoid passing account_id on every call.",
            json!({}),
            vec![],
        ),
        tool_def(
            "get_account",
            "Get details of a specific sub-account.",
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" }
            }),
            vec![],
        ),
        tool_def(
            "create_account",
            "Create a new sub-account. Use keygen=true to generate a P256 key and create a Turnkey-backed account.",
            json!({
                "keygen": { "type": "boolean", "description": "Generate a P256 key and create a turnkey_p256 account (default: true)" },
                "use_file_key": { "type": "boolean", "description": "Use file-based key instead of the default (iCloud Keychain on macOS brew builds)" },
                "signer_type": { "type": "string", "description": "Signer type when not using keygen: \"eoa\" or \"turnkey_p256\"" },
                "ethereum_signer": { "type": "string", "description": "Ethereum address (for eoa accounts)" }
            }),
            vec![],
        ),
        // --- Data & reference ---
        tool_def(
            "get_portfolio",
            concat!(
                "Get portfolio data for an account. Legend is a yield platform — always show APRs alongside yield positions ",
                "(e.g. \"$5 earning 5.0% on Morpho, World Chain\", not just \"$5 on Morpho\"). ",
                "Default section is \"balances,yield_markets,prices\" which returns holdings, APRs, and USD prices in one call. ",
                "Use filter (regex on keys) to narrow results.\n\n",
                "Sections and their shapes:\n\n",
                "balances — token holdings and yield positions per wallet\n",
                "  Key: \"token/{network}/{symbol}/{wallet}\" → Value: amount as scientific string\n",
                "  Key: \"yield_market/{protocol}/{network}/{address}/{asset}/{wallet}\" → Value: amount\n",
                "  Example: {\"token/base/USDC/0x842d...\": \"300.25e6\", \"yield_market/morpho_vault/world_chain/0xb1e8.../USDC/0x842d...\": \"5e6\"}\n\n",
                "prices — USD token prices\n",
                "  Key: \"token/{SYMBOL}\" → Value: price string\n",
                "  Example: {\"token/USDC\": \"1\", \"token/WETH\": \"2713.04\"}\n\n",
                "yield_markets — available yield protocols with APRs\n",
                "  Key: \"{protocol}/{network}/{address}/{asset}\" → Value: {supply_apr, supply_rewards_apr, supply_cap, total_supply}\n",
                "  Protocol prefixes: comet (=compound), aave, morpho_vault\n",
                "  Example: {\"morpho_vault/world_chain/0xb1e8.../USDC\": {\"supply_apr\": \"0.005\", \"supply_rewards_apr\": \"0.044\", ...}}\n\n",
                "borrow_markets — borrowing rates and collateral info\n",
                "  Key: \"{protocol}/{network}/{address}/{asset}\" → Value: {borrow_apr, borrow_rewards_apr, total_borrow, collaterals: {SYMBOL: {borrow_collateral_factor, usd_price, ...}}}\n\n",
                "rewards — unclaimed protocol rewards\n",
                "  Key: \"{type}/{network}/...\" → Value: reward proof info\n\n",
                "all — entire folio (large! use with filter)\n\n",
                "Amounts use scientific notation: \"300.25e6\" = 300.25 (6 decimals), \"1.5e18\" = 1.5 (18 decimals).\n",
                "filter examples: \"USDC\" (all USDC entries), \"world_chain\" (all World Chain), \"morpho_vault.*USDC\" (Morpho USDC vaults).\n\n",
                "Tip: when in doubt, use section \"balances,yield_markets,prices\" for a comprehensive one-shot view. ",
                "This gives you balances, current APRs to present alongside yield positions, and prices for non-stablecoin assets. ",
                "Match yield_market balance keys (yield_market/{protocol}/{network}/{address}/{asset}/{wallet}) to yield_markets APR keys ",
                "({protocol}/{network}/{address}/{asset}) to show users what each position is earning.\n\n",
                "Note: Legend automatically bridges funds between chains, so users think in terms of total asset balances, ",
                "not per-chain amounts. When presenting balances to users, sum token amounts across chains by asset symbol ",
                "(e.g. \"You have $10 USDC total\") and group yield positions separately with their APRs ",
                "(e.g. \"$5 earning 5.0% on Morpho, World Chain\"). ",
                "The per-chain detail is available but should not be the primary presentation."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "section": { "type": "string", "description": "One section name, comma-separated names, or \"all\". Default: \"balances,yield_markets,prices\". Use \"balances\" alone if you only need holdings." },
                "filter": { "type": "string", "description": "Case-insensitive regex applied to keys. Only entries whose key matches are returned." }
            }),
            vec![],
        ),
        tool_def(
            "get_activities",
            "Get transaction history for an account. Pass activity_id (e.g. \"act_xxx\") to get a single activity — use this to check execution status. Without activity_id, returns all recent activities.",
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "activity_id": { "type": "string", "description": "Fetch a single activity by external ID (e.g. \"act_xxx\")." }
            }),
            vec![],
        ),
        tool_def(
            "list_networks",
            "List all supported blockchain networks.",
            json!({}),
            vec![],
        ),
        tool_def(
            "list_assets",
            "List all supported assets with decimals and network availability.",
            json!({}),
            vec![],
        ),
        tool_def(
            "list_markets",
            concat!(
                "List all supported on-chain markets across Morpho, Aave, and Compound.\n\n",
                "Returns an array of market objects, each with a \"protocol\" field:\n\n",
                "morpho_market — Morpho lending markets (for borrow, loop_long, unloop_long, add_backing, withdraw_backing):\n",
                "  market_id: 0x-prefixed 32-byte identifier (pass to leverage/borrow tools)\n",
                "  loan_token, collateral_token: token addresses\n",
                "  lltv: liquidation loan-to-value\n\n",
                "morpho_vault — Morpho yield vaults (for earn, withdraw, migrate, swap_and_supply):\n",
                "  vault: vault address (pass as \"market\" param to earn/withdraw tools)\n",
                "  name, symbol, asset: vault metadata\n\n",
                "aave_market — Aave lending pools with reserves\n\n",
                "comet — Compound v3 markets with collateral assets"
            ),
            json!({}),
            vec![],
        ),
        // --- Action tools ---
        tool_def(
            "earn",
            concat!(
                "Deposit assets into a yield-earning protocol. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Examples:\n",
                "  1 USDC into Compound on Base:            {amount: \"1000000\", asset: \"USDC\", network: \"base\", protocol: \"compound\"}\n",
                "  Max USDC into Aave on Optimism:          {amount: \"max\", asset: \"USDC\", network: \"optimism\", protocol: \"aave\"}\n",
                "  5 USDC into Morpho vault on World Chain:  {amount: \"5000000\", asset: \"USDC\", network: \"world_chain\", protocol: \"morpho_vault\", market: \"0xb1e8...\"}\n\n",
                "amount: smallest unit (\"1000000\" = 1 USDC) or \"max\" for full available balance.\n",
                "protocol: \"compound\", \"aave\", or \"morpho_vault\". Morpho requires the market (vault address) parameter.\n",
                "execute (default true): set false to only create the plan (returns plan_id + digest for external signing via execute_plan).\n",
                "wait (default true): block until the activity completes or fails. Only applies when execute=true."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "amount": { "type": "string", "description": "Amount in smallest unit, or \"max\" for full balance" },
                "asset": { "type": "string", "description": "Asset symbol (e.g. \"USDC\")" },
                "network": { "type": "string", "description": "Target network (e.g. \"base\", \"optimism\", \"world_chain\")" },
                "protocol": { "type": "string", "description": "\"compound\", \"aave\", or \"morpho_vault\"" },
                "market": { "type": "string", "description": "Vault address — required for morpho_vault only (e.g. \"0xb1e80387ebe53ff75a89736097d34dc8d9e9045b\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["amount", "asset", "network", "protocol"],
        ),
        tool_def(
            "withdraw",
            concat!(
                "Withdraw assets from a yield-earning protocol. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Examples:\n",
                "  Withdraw 1 USDC from Compound on Base:  {amount: \"1000000\", asset: \"USDC\", network: \"base\", protocol: \"compound\"}\n",
                "  Withdraw max from Aave on Optimism:     {amount: \"max\", asset: \"USDC\", network: \"optimism\", protocol: \"aave\"}\n\n",
                "amount: smallest unit (\"1000000\" = 1 USDC) or \"max\" to withdraw entire position (no dust left behind).\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "amount": { "type": "string", "description": "Amount in smallest unit, or \"max\" for full withdrawal" },
                "asset": { "type": "string", "description": "Asset symbol (e.g. \"USDC\")" },
                "network": { "type": "string", "description": "Network where the position is (e.g. \"base\", \"optimism\")" },
                "protocol": { "type": "string", "description": "\"compound\", \"aave\", or \"morpho_vault\"" },
                "market": { "type": "string", "description": "Vault address — required for morpho_vault only" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["amount", "asset", "network", "protocol"],
        ),
        tool_def(
            "swap",
            concat!(
                "Swap one asset for another. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Provide exactly one of sell_amount or buy_amount (not both):\n",
                "  Exact input  — sell_amount: sell this exact amount, receive whatever the market gives.\n",
                "  Exact output — buy_amount: buy this exact amount, sell whatever the market requires.\n\n",
                "Examples:\n",
                "  Sell 1 USDC for WETH on Base:   {sell_asset: \"USDC\", buy_asset: \"WETH\", sell_amount: \"1000000\", network: \"base\"}\n",
                "  Buy 0.5 WETH with USDC on Base: {sell_asset: \"USDC\", buy_asset: \"WETH\", buy_amount: \"500000000000000000\", network: \"base\"}\n\n",
                "Tip: use execute=false to get a quote first — the plan response includes expected output amounts and pricing. ",
                "Then call execute_plan to proceed if the quote looks good.\n\n",
                "execute (default true): set false to only create the plan (acts as a quote).\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "sell_asset": { "type": "string", "description": "Asset to sell (e.g. \"USDC\")" },
                "buy_asset": { "type": "string", "description": "Asset to buy (e.g. \"WETH\")" },
                "sell_amount": { "type": "string", "description": "Amount to sell in smallest unit (mutually exclusive with buy_amount)" },
                "buy_amount": { "type": "string", "description": "Amount to buy in smallest unit (mutually exclusive with sell_amount)" },
                "network": { "type": "string", "description": "Network (e.g. \"base\", \"ethereum\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["sell_asset", "buy_asset", "network"],
        ),
        tool_def(
            "transfer",
            concat!(
                "Transfer assets to a recipient address. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Example:\n",
                "  Send 1 USDC on Base: {amount: \"1000000\", asset: \"USDC\", network: \"base\", recipient: \"0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18\"}\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "amount": { "type": "string", "description": "Amount in smallest unit, or \"max\" for full balance" },
                "asset": { "type": "string", "description": "Asset symbol (e.g. \"USDC\")" },
                "network": { "type": "string", "description": "Network (e.g. \"base\", \"ethereum\")" },
                "recipient": { "type": "string", "description": "Recipient's 0x-prefixed address" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["amount", "asset", "network", "recipient"],
        ),
        tool_def(
            "borrow",
            concat!(
                "Borrow assets against collateral. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Example:\n",
                "  Borrow 0.1 USDC with 0.001 WETH collateral on Compound (Base):\n",
                "    {amount: \"100000\", asset: \"USDC\", collateral_amount: \"1000000000000000\", collateral_asset: \"WETH\", network: \"base\", protocol: \"compound\"}\n\n",
                "protocol: \"compound\" or \"morpho\". Morpho requires the market parameter (32-byte market_id).\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "amount": { "type": "string", "description": "Amount to borrow in smallest unit" },
                "asset": { "type": "string", "description": "Asset to borrow (e.g. \"USDC\")" },
                "collateral_amount": { "type": "string", "description": "Collateral amount in smallest unit" },
                "collateral_asset": { "type": "string", "description": "Collateral asset (e.g. \"WETH\")" },
                "network": { "type": "string", "description": "Network (e.g. \"base\")" },
                "protocol": { "type": "string", "description": "\"compound\" or \"morpho\"" },
                "market": { "type": "string", "description": "32-byte market_id — required for morpho only" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["amount", "asset", "collateral_amount", "collateral_asset", "network", "protocol"],
        ),
        tool_def(
            "repay",
            concat!(
                "Repay borrowed assets and reclaim collateral. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Example:\n",
                "  Repay 0.1 USDC and withdraw 0.001 WETH collateral on Compound (Base):\n",
                "    {amount: \"100000\", asset: \"USDC\", collateral_amount: \"1000000000000000\", collateral_asset: \"WETH\", network: \"base\", protocol: \"compound\"}\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "amount": { "type": "string", "description": "Amount to repay in smallest unit" },
                "asset": { "type": "string", "description": "Asset to repay (e.g. \"USDC\")" },
                "collateral_amount": { "type": "string", "description": "Collateral to withdraw in smallest unit" },
                "collateral_asset": { "type": "string", "description": "Collateral asset (e.g. \"WETH\")" },
                "network": { "type": "string", "description": "Network (e.g. \"base\")" },
                "protocol": { "type": "string", "description": "\"compound\" or \"morpho\"" },
                "market": { "type": "string", "description": "32-byte market_id — required for morpho only" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["amount", "asset", "collateral_amount", "collateral_asset", "network", "protocol"],
        ),
        tool_def(
            "migrate",
            concat!(
                "Move a yield position from one protocol to another in a single step (withdraws + re-supplies atomically). ",
                "Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Examples:\n",
                "  Migrate 1 USDC from Compound to Aave on Base:\n",
                "    {amount: \"1000000\", asset: \"USDC\", from_protocol: \"compound\", to_protocol: \"aave\", network: \"base\"}\n",
                "  Migrate max USDC from Aave to Morpho vault on World Chain:\n",
                "    {amount: \"max\", asset: \"USDC\", from_protocol: \"aave\", to_protocol: \"morpho_vault\", network: \"world_chain\", to_market: \"0xb1e8...\"}\n\n",
                "protocol values: \"compound\", \"aave\", or \"morpho_vault\".\n",
                "from_market / to_market: required when the corresponding protocol is morpho_vault (pass the vault address).\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "amount": { "type": "string", "description": "Amount in smallest unit, or \"max\" for full position" },
                "asset": { "type": "string", "description": "Asset symbol (e.g. \"USDC\")" },
                "from_protocol": { "type": "string", "description": "Source protocol: \"compound\", \"aave\", or \"morpho_vault\"" },
                "to_protocol": { "type": "string", "description": "Destination protocol: \"compound\", \"aave\", or \"morpho_vault\"" },
                "network": { "type": "string", "description": "Network (e.g. \"base\", \"world_chain\")" },
                "from_market": { "type": "string", "description": "Vault address — required when from_protocol is morpho_vault" },
                "to_market": { "type": "string", "description": "Vault address — required when to_protocol is morpho_vault" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["amount", "asset", "from_protocol", "to_protocol", "network"],
        ),
        tool_def(
            "swap_and_supply",
            concat!(
                "Swap one asset for another and deposit the result into a yield protocol — all in one step. ",
                "Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Examples:\n",
                "  Sell 1 USDC for WETH and supply to Aave on Base:\n",
                "    {sell_asset: \"USDC\", sell_amount: \"1000000\", buy_asset: \"WETH\", protocol: \"aave\", network: \"base\"}\n",
                "  Sell max WETH for USDC and supply to Morpho vault on World Chain:\n",
                "    {sell_asset: \"WETH\", sell_amount: \"max\", buy_asset: \"USDC\", protocol: \"morpho_vault\", network: \"world_chain\", market: \"0xb1e8...\"}\n\n",
                "protocol: \"compound\", \"aave\", or \"morpho_vault\". Morpho requires the market (vault address) parameter.\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "sell_asset": { "type": "string", "description": "Asset to sell (e.g. \"USDC\")" },
                "sell_amount": { "type": "string", "description": "Amount to sell in smallest unit, or \"max\" for full balance" },
                "buy_asset": { "type": "string", "description": "Asset to buy and supply (e.g. \"WETH\")" },
                "protocol": { "type": "string", "description": "\"compound\", \"aave\", or \"morpho_vault\"" },
                "network": { "type": "string", "description": "Network (e.g. \"base\", \"world_chain\")" },
                "market": { "type": "string", "description": "Vault address — required for morpho_vault only" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["sell_asset", "sell_amount", "buy_asset", "protocol", "network"],
        ),
        // --- Rewards ---
        tool_def(
            "claim_rewards",
            concat!(
                "Claim unclaimed protocol rewards for an asset. Creates a plan, signs, executes, and waits for completion by default.\n\n",
                "Use the portfolio tool with section \"rewards\" to see unclaimed rewards before calling this.\n\n",
                "Example:\n",
                "  Claim USDC rewards: {asset: \"USDC\"}\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "asset": { "type": "string", "description": "Asset symbol to claim rewards for (e.g. \"USDC\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["asset"],
        ),
        tool_def(
            "reinvest_rewards",
            concat!(
                "Claim protocol rewards and reinvest them back into a yield position (auto-compound). ",
                "Claims each reward asset, swaps them to the target asset, and re-supplies to the protocol.\n\n",
                "Use the portfolio tool with section \"rewards\" to see available reward assets before calling this.\n\n",
                "Example:\n",
                "  Reinvest COMP rewards back into USDC on Compound (Base):\n",
                "    {asset: \"USDC\", protocol: \"compound\", network: \"base\", reward_assets: [\"COMP\"]}\n\n",
                "protocol: \"compound\", \"aave\", or \"morpho_vault\". Morpho requires the market (vault address) parameter.\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "asset": { "type": "string", "description": "Target asset to reinvest into (e.g. \"USDC\")" },
                "protocol": { "type": "string", "description": "\"compound\", \"aave\", or \"morpho_vault\"" },
                "network": { "type": "string", "description": "Network (e.g. \"base\", \"world_chain\")" },
                "reward_assets": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of reward asset symbols to claim and reinvest (e.g. [\"COMP\", \"WETH\"])"
                },
                "market": { "type": "string", "description": "Vault address — required when protocol is morpho_vault" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["asset", "protocol", "network", "reward_assets"],
        ),
        // --- Leverage tools ---
        tool_def(
            "loop_long",
            concat!(
                "Create or increase a leveraged long position on Morpho via looping. ",
                "Borrows backing asset, swaps to exposure asset, and supplies as collateral — repeated to achieve leverage.\n\n",
                "All amounts in smallest unit. market_id is a 0x-prefixed 32-byte Morpho market identifier.\n\n",
                "Parameters:\n",
                "  exposure_asset: Asset to go long on (e.g. \"WETH\")\n",
                "  backing_asset: Asset to borrow (e.g. \"USDC\")\n",
                "  market_id: 0x-prefixed 32-byte Morpho market ID\n",
                "  is_increase: true to increase position, false to create new\n",
                "  exposure_amount: Amount of exposure asset in smallest unit\n",
                "  max_swap_backing_amount: Maximum backing to swap per iteration\n",
                "  max_provided_backing_amount: Maximum backing to provide from wallet\n",
                "  pool_fee: Uniswap pool fee tier (e.g. 500, 3000, 10000)\n",
                "  network: Network (e.g. \"base\")\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "exposure_asset": { "type": "string", "description": "Asset to go long on (e.g. \"WETH\")" },
                "backing_asset": { "type": "string", "description": "Asset to borrow against (e.g. \"USDC\")" },
                "market_id": { "type": "string", "description": "0x-prefixed 32-byte Morpho market ID" },
                "is_increase": { "type": "boolean", "description": "true to add to existing position, false for new position" },
                "exposure_amount": { "type": "string", "description": "Exposure amount in smallest unit" },
                "max_swap_backing_amount": { "type": "string", "description": "Max backing amount to swap per loop iteration" },
                "max_provided_backing_amount": { "type": "string", "description": "Max backing amount to provide from wallet" },
                "pool_fee": { "type": "integer", "description": "Uniswap pool fee tier (500 = 0.05%, 3000 = 0.3%, 10000 = 1%)" },
                "network": { "type": "string", "description": "Network (e.g. \"base\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["exposure_asset", "backing_asset", "market_id", "is_increase", "exposure_amount",
                 "max_swap_backing_amount", "max_provided_backing_amount", "pool_fee", "network"],
        ),
        tool_def(
            "unloop_long",
            concat!(
                "Unwind (reduce or close) a leveraged long position on Morpho. ",
                "Withdraws collateral, swaps back to backing asset, and repays debt.\n\n",
                "All amounts in smallest unit. market_id is a 0x-prefixed 32-byte Morpho market identifier.\n\n",
                "Parameters:\n",
                "  exposure_asset: The long asset (e.g. \"WETH\")\n",
                "  backing_asset: The borrowed asset (e.g. \"USDC\")\n",
                "  market_id: 0x-prefixed 32-byte Morpho market ID\n",
                "  exposure_amount: Amount of exposure to unwind\n",
                "  backing_amount_to_exit: Backing amount to repay\n",
                "  min_swap_backing_amount: Minimum acceptable backing from swap (slippage protection)\n",
                "  pool_fee: Uniswap pool fee tier (e.g. 500, 3000, 10000)\n",
                "  network: Network (e.g. \"base\")\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "exposure_asset": { "type": "string", "description": "The long asset (e.g. \"WETH\")" },
                "backing_asset": { "type": "string", "description": "The borrowed asset (e.g. \"USDC\")" },
                "market_id": { "type": "string", "description": "0x-prefixed 32-byte Morpho market ID" },
                "exposure_amount": { "type": "string", "description": "Amount of exposure to unwind in smallest unit" },
                "backing_amount_to_exit": { "type": "string", "description": "Backing amount to repay in smallest unit" },
                "min_swap_backing_amount": { "type": "string", "description": "Minimum acceptable backing from swap (slippage protection)" },
                "pool_fee": { "type": "integer", "description": "Uniswap pool fee tier (500 = 0.05%, 3000 = 0.3%, 10000 = 1%)" },
                "network": { "type": "string", "description": "Network (e.g. \"base\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["exposure_asset", "backing_asset", "market_id", "exposure_amount",
                 "backing_amount_to_exit", "min_swap_backing_amount", "pool_fee", "network"],
        ),
        tool_def(
            "add_backing",
            concat!(
                "Add backing (collateral) to an existing Morpho leveraged position. ",
                "Reduces liquidation risk by increasing the collateral ratio.\n\n",
                "Parameters:\n",
                "  exposure_asset: The position's exposure asset (e.g. \"WETH\")\n",
                "  backing_asset: The collateral/backing asset (e.g. \"USDC\")\n",
                "  market_id: 0x-prefixed 32-byte Morpho market ID\n",
                "  amount: Backing amount to add in smallest unit\n",
                "  is_short: true if this is a short position, false for long\n",
                "  network: Network (e.g. \"base\")\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "exposure_asset": { "type": "string", "description": "Position's exposure asset (e.g. \"WETH\")" },
                "backing_asset": { "type": "string", "description": "Collateral/backing asset (e.g. \"USDC\")" },
                "market_id": { "type": "string", "description": "0x-prefixed 32-byte Morpho market ID" },
                "amount": { "type": "string", "description": "Amount of backing to add in smallest unit" },
                "is_short": { "type": "boolean", "description": "true for short position, false for long" },
                "network": { "type": "string", "description": "Network (e.g. \"base\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["exposure_asset", "backing_asset", "market_id", "amount", "is_short", "network"],
        ),
        tool_def(
            "withdraw_backing",
            concat!(
                "Withdraw backing (collateral) from an existing Morpho leveraged position. ",
                "Increases liquidation risk — use carefully.\n\n",
                "Parameters:\n",
                "  exposure_asset: The position's exposure asset (e.g. \"WETH\")\n",
                "  backing_asset: The collateral/backing asset (e.g. \"USDC\")\n",
                "  market_id: 0x-prefixed 32-byte Morpho market ID\n",
                "  amount: Backing amount to withdraw in smallest unit\n",
                "  is_short: true if this is a short position, false for long\n",
                "  network: Network (e.g. \"base\")\n\n",
                "execute (default true): set false to only create the plan.\n",
                "wait (default true): block until the activity completes or fails."
            ),
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "exposure_asset": { "type": "string", "description": "Position's exposure asset (e.g. \"WETH\")" },
                "backing_asset": { "type": "string", "description": "Collateral/backing asset (e.g. \"USDC\")" },
                "market_id": { "type": "string", "description": "0x-prefixed 32-byte Morpho market ID" },
                "amount": { "type": "string", "description": "Amount of backing to withdraw in smallest unit" },
                "is_short": { "type": "boolean", "description": "true for short position, false for long" },
                "network": { "type": "string", "description": "Network (e.g. \"base\")" },
                "execute": { "type": "boolean", "description": "Sign and execute the plan (default: true)" },
                "wait": { "type": "boolean", "description": "Wait for terminal status (default: true)" }
            }),
            vec!["exposure_asset", "backing_asset", "market_id", "amount", "is_short", "network"],
        ),
        // --- Low-level (for execute=false flow) ---
        tool_def(
            "execute_plan",
            "Execute a previously created plan with an external signature. Only needed when action tools (earn, withdraw, swap, etc.) were called with execute=false.",
            json!({
                "account_id": { "type": "string", "description": "Account ID (optional if set_account was called)" },
                "plan_id": { "type": "string", "description": "Plan ID returned by the action tool" },
                "signature": { "type": "string", "description": "0x-prefixed EIP-712 signature over the plan digest" }
            }),
            vec!["plan_id", "signature"],
        ),
    ]
}

fn tool_def(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }
    })
}

// --- Tool dispatch ---

async fn handle_tool_call(
    name: &str,
    args: Value,
    session: &mut McpSession,
) -> anyhow::Result<String> {
    match name {
        "login" => {
            let base_url = resolve_base_url(&None, session.env);
            crate::commands::login::login(&base_url, session.env, &session.profile).await?;
            Ok(
                json!({"status": "logged_in", "profile": session.profile, "env": session.env.dir_name()})
                    .to_string(),
            )
        }

        "whoami" => {
            let client = make_client(session)?;
            let pa = client.prime_account().await?;
            Ok(serde_json::to_string(&pa)?)
        }

        "set_account" => {
            let id = str_arg(&args, "account_id")?;
            // Validate the account exists
            let client = make_client(session)?;
            let account = client.accounts.get(&id).await?;
            session.active_account_id = Some(id.clone());
            // Persist to profile so it survives server restarts
            let mut p = config::load_profile(session.env, &session.profile)
                .ok_or_else(|| anyhow::anyhow!("Profile not found; cannot persist account selection"))?;
            p.account_external_id = id.clone();
            config::save_profile(session.env, &session.profile, &p)?;
            Ok(json!({
                "status": "active_account_set",
                "account_id": id,
                "ethereum_signer_address": account.ethereum_signer_address,
                "legend_wallet_address": account.legend_wallet_address,
                "solana_wallet_address": account.solana_wallet_address,
            })
            .to_string())
        }

        "list_accounts" => {
            let client = make_client(session)?;
            let list = client.accounts.list().await?;
            Ok(serde_json::to_string(&list)?)
        }

        "get_account" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let account = client.accounts.get(&id).await?;
            Ok(serde_json::to_string(&account)?)
        }

        "create_account" => {
            let keygen = args.get("keygen").and_then(|v| v.as_bool()).unwrap_or(true);
            if keygen {
                let use_file_key = args
                    .get("use_file_key")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let client = make_client(session)?;

                let (signer, key_source, key_label, key_path) =
                    generate_key(&session.profile, use_file_key, session.env)?;

                let account = client
                    .accounts
                    .create(&CreateAccountParams {
                        signer_type: "turnkey_p256".into(),
                        p256_public_key: Some(signer.public_key_hex().to_string()),
                        key_storage: Some(key_source.clone()),
                        ..Default::default()
                    })
                    .await?;

                let qk = resolve_query_key(&session.key, session.env, &session.profile)
                    .map_err(anyhow::Error::msg)?;
                let p = config::Profile {
                    query_key: Some(qk),
                    key_source,
                    key_label,
                    key_path,
                    p256_public_key: signer.public_key_hex().to_string(),
                    sub_org_id: account.turnkey_sub_org_id.clone().unwrap_or_default(),
                    ethereum_signer_address: account
                        .ethereum_signer_address
                        .clone()
                        .unwrap_or_default(),
                    account_external_id: account.account_id.clone(),
                };
                config::save_profile(session.env, &session.profile, &p)?;

                // Auto-set as active account
                session.active_account_id = Some(account.account_id.clone());

                Ok(serde_json::to_string(&account)?)
            } else {
                let client = make_client(session)?;
                let signer_type = args
                    .get("signer_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("eoa");
                let account = client
                    .accounts
                    .create(&CreateAccountParams {
                        signer_type: signer_type.into(),
                        ethereum_signer_address: args
                            .get("ethereum_signer")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        ..Default::default()
                    })
                    .await?;
                Ok(serde_json::to_string(&account)?)
            }
        }

        "get_portfolio" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let section = args
                .get("section")
                .and_then(|v| v.as_str())
                .unwrap_or("balances,yield_markets,prices");
            let filter_pat = opt_str(&args, "filter");

            let folio = client
                .accounts
                .folio(&id, &FolioOpts { cached: false })
                .await?;

            let folio_obj = &folio.folio;

            let section_data = if section == "all" {
                folio_obj.clone()
            } else if section.contains(',') {
                // Multi-section: "balances,yield_markets" → {"balances": {...}, "yield_markets": {...}}
                let mut result = serde_json::Map::new();
                for s in section.split(',') {
                    let s = s.trim();
                    match folio_obj.get(s) {
                        Some(data) => { result.insert(s.to_string(), data.clone()); }
                        None => {
                            let valid_keys: Vec<&str> = folio_obj
                                .as_object()
                                .map(|m| m.keys().map(|k| k.as_str()).collect())
                                .unwrap_or_default();
                            anyhow::bail!(
                                "Unknown section \"{s}\". Valid sections: {}",
                                valid_keys.join(", ")
                            );
                        }
                    }
                }
                serde_json::Value::Object(result)
            } else {
                match folio_obj.get(section) {
                    Some(data) => data.clone(),
                    None => {
                        let valid_keys: Vec<&str> = folio_obj
                            .as_object()
                            .map(|m| m.keys().map(|k| k.as_str()).collect())
                            .unwrap_or_default();
                        anyhow::bail!(
                            "Unknown section \"{section}\". Valid sections: {}",
                            valid_keys.join(", ")
                        );
                    }
                }
            };

            if let Some(pat) = filter_pat {
                let re = regex::RegexBuilder::new(&pat)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Invalid regex: {e}"))?;

                match section_data {
                    serde_json::Value::Object(map) => {
                        let filtered: serde_json::Map<String, serde_json::Value> =
                            map.into_iter().filter(|(k, _)| re.is_match(k)).collect();
                        Ok(serde_json::to_string(&serde_json::Value::Object(filtered))?)
                    }
                    other => Ok(serde_json::to_string(&other)?),
                }
            } else {
                Ok(serde_json::to_string(&section_data)?)
            }
        }

        "get_activities" => {
            let client = make_client(session)?;
            let account_id = resolve_account_id(&args, session)?;

            if let Some(activity_id) = opt_str(&args, "activity_id") {
                let activity = client
                    .accounts
                    .activity_by_id(&account_id, &activity_id)
                    .await?;
                Ok(serde_json::to_string(&activity)?)
            } else {
                let list = client.accounts.activities(&account_id).await?;
                Ok(serde_json::to_string(&list)?)
            }
        }

        "list_networks" => {
            let client = make_client(session)?;
            let networks = client.networks().await?;
            Ok(serde_json::to_string(&networks)?)
        }

        "list_assets" => {
            let client = make_client(session)?;
            let assets = client.assets().await?;
            Ok(serde_json::to_string(&assets)?)
        }

        "list_markets" => {
            let client = make_client(session)?;
            let markets = client.markets().await?;
            Ok(serde_json::to_string(&markets)?)
        }

        // --- Action tools ---

        "earn" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .earn(
                    &id,
                    &EarnParams {
                        amount: resolve_amount(&args, "amount")?,
                        asset: str_arg(&args, "asset")?,
                        network: str_arg(&args, "network")?,
                        protocol: str_arg(&args, "protocol")?,
                        market: opt_str(&args, "market"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "earn", &args, session).await
        }

        "withdraw" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .withdraw(
                    &id,
                    &WithdrawParams {
                        amount: resolve_amount(&args, "amount")?,
                        asset: str_arg(&args, "asset")?,
                        network: str_arg(&args, "network")?,
                        protocol: str_arg(&args, "protocol")?,
                        market: opt_str(&args, "market"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "withdraw", &args, session).await
        }

        "swap" => {
            if opt_str(&args, "sell_amount").is_none() && opt_str(&args, "buy_amount").is_none() {
                anyhow::bail!("swap requires exactly one of sell_amount or buy_amount");
            }
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .swap(
                    &id,
                    &SwapParams {
                        sell_asset: str_arg(&args, "sell_asset")?,
                        buy_asset: str_arg(&args, "buy_asset")?,
                        network: str_arg(&args, "network")?,
                        sell_amount: opt_str(&args, "sell_amount"),
                        buy_amount: opt_str(&args, "buy_amount"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "swap", &args, session).await
        }

        "transfer" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .transfer(
                    &id,
                    &TransferParams {
                        amount: resolve_amount(&args, "amount")?,
                        asset: str_arg(&args, "asset")?,
                        network: str_arg(&args, "network")?,
                        recipient: str_arg(&args, "recipient")?,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "transfer", &args, session).await
        }

        "borrow" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .borrow(
                    &id,
                    &BorrowParams {
                        amount: resolve_amount(&args, "amount")?,
                        asset: str_arg(&args, "asset")?,
                        network: str_arg(&args, "network")?,
                        protocol: str_arg(&args, "protocol")?,
                        collateral_amount: str_arg(&args, "collateral_amount")?,
                        collateral_asset: str_arg(&args, "collateral_asset")?,
                        market: opt_str(&args, "market"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "borrow", &args, session).await
        }

        "repay" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .repay(
                    &id,
                    &RepayParams {
                        amount: resolve_amount(&args, "amount")?,
                        asset: str_arg(&args, "asset")?,
                        network: str_arg(&args, "network")?,
                        protocol: str_arg(&args, "protocol")?,
                        collateral_amount: str_arg(&args, "collateral_amount")?,
                        collateral_asset: str_arg(&args, "collateral_asset")?,
                        market: opt_str(&args, "market"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "repay", &args, session).await
        }

        "migrate" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .migrate(
                    &id,
                    &MigrateParams {
                        amount: resolve_amount(&args, "amount")?,
                        asset: str_arg(&args, "asset")?,
                        from_protocol: str_arg(&args, "from_protocol")?,
                        to_protocol: str_arg(&args, "to_protocol")?,
                        network: str_arg(&args, "network")?,
                        from_market: opt_str(&args, "from_market"),
                        to_market: opt_str(&args, "to_market"),
                        migrate_only_supply_balances: None,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "migrate", &args, session).await
        }

        "swap_and_supply" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .swap_and_supply(
                    &id,
                    &SwapAndSupplyParams {
                        sell_asset: str_arg(&args, "sell_asset")?,
                        sell_amount: resolve_amount(&args, "sell_amount")?,
                        buy_asset: str_arg(&args, "buy_asset")?,
                        protocol: str_arg(&args, "protocol")?,
                        network: str_arg(&args, "network")?,
                        market: opt_str(&args, "market"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "swap_and_supply", &args, session).await
        }

        "claim_rewards" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .claim_rewards(
                    &id,
                    &ClaimRewardsParams {
                        asset: str_arg(&args, "asset")?,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "claim_rewards", &args, session).await
        }

        "reinvest_rewards" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .reinvest_rewards(
                    &id,
                    &ReinvestRewardsParams {
                        asset: str_arg(&args, "asset")?,
                        protocol: str_arg(&args, "protocol")?,
                        network: str_arg(&args, "network")?,
                        reward_assets: str_array_arg(&args, "reward_assets")?,
                        market: opt_str(&args, "market"),
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "reinvest_rewards", &args, session).await
        }

        "loop_long" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .loop_long(
                    &id,
                    &LoopLongParams {
                        exposure_asset: str_arg(&args, "exposure_asset")?,
                        backing_asset: str_arg(&args, "backing_asset")?,
                        market_id: str_arg(&args, "market_id")?,
                        is_increase: bool_arg(&args, "is_increase")?,
                        exposure_amount: str_arg(&args, "exposure_amount")?,
                        max_swap_backing_amount: str_arg(&args, "max_swap_backing_amount")?,
                        max_provided_backing_amount: str_arg(&args, "max_provided_backing_amount")?,
                        pool_fee: u64_arg(&args, "pool_fee")?,
                        network: str_arg(&args, "network")?,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "loop_long", &args, session).await
        }

        "unloop_long" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .unloop_long(
                    &id,
                    &UnloopLongParams {
                        exposure_asset: str_arg(&args, "exposure_asset")?,
                        backing_asset: str_arg(&args, "backing_asset")?,
                        market_id: str_arg(&args, "market_id")?,
                        exposure_amount: str_arg(&args, "exposure_amount")?,
                        backing_amount_to_exit: str_arg(&args, "backing_amount_to_exit")?,
                        min_swap_backing_amount: str_arg(&args, "min_swap_backing_amount")?,
                        pool_fee: u64_arg(&args, "pool_fee")?,
                        network: str_arg(&args, "network")?,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "unloop_long", &args, session).await
        }

        "add_backing" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .add_backing(
                    &id,
                    &AddBackingParams {
                        exposure_asset: str_arg(&args, "exposure_asset")?,
                        backing_asset: str_arg(&args, "backing_asset")?,
                        market_id: str_arg(&args, "market_id")?,
                        amount: str_arg(&args, "amount")?,
                        is_short: bool_arg(&args, "is_short")?,
                        network: str_arg(&args, "network")?,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "add_backing", &args, session).await
        }

        "withdraw_backing" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan = client
                .plan
                .withdraw_backing(
                    &id,
                    &WithdrawBackingParams {
                        exposure_asset: str_arg(&args, "exposure_asset")?,
                        backing_asset: str_arg(&args, "backing_asset")?,
                        market_id: str_arg(&args, "market_id")?,
                        amount: str_arg(&args, "amount")?,
                        is_short: bool_arg(&args, "is_short")?,
                        network: str_arg(&args, "network")?,
                    },
                )
                .await?;
            finish_action(&client, plan, &id, "withdraw_backing", &args, session).await
        }

        "execute_plan" => {
            let client = make_client(session)?;
            let id = resolve_account_id(&args, session)?;
            let plan_id = str_arg(&args, "plan_id")?;
            let signature = str_arg(&args, "signature")?;
            let result = client
                .plan
                .execute(&id, &ExecuteParams { plan_id, signature })
                .await?;
            Ok(serde_json::to_string(&result)?)
        }

        _ => anyhow::bail!("Unknown tool: {name}"),
    }
}

// --- Action helpers ---

/// Shared logic for all action tools: optionally sign+execute and optionally wait.
async fn finish_action(
    client: &LegendPrime,
    plan: Plan,
    account_id: &str,
    action: &str,
    args: &Value,
    session: &McpSession,
) -> anyhow::Result<String> {
    let execute = args.get("execute").and_then(|v| v.as_bool()).unwrap_or(true);

    if !execute {
        // Return plan for external signing
        return Ok(json!({
            "plan_id": plan.plan_id,
            "digest": plan.digest(),
            "expires_at": plan.expires_at,
            "action": action,
            "note": "Plan created but not executed. Sign the digest and call execute_plan to proceed."
        })
        .to_string());
    }

    // Sign
    let digest = plan
        .digest()
        .ok_or_else(|| anyhow::anyhow!("Plan response missing digest"))?;
    eprintln!("[legend] Signing plan {}...", plan.plan_id);
    let signature = sign_with_profile(session.env, &session.profile, digest).await?;

    // Execute
    eprintln!("[legend] Executing plan {}...", plan.plan_id);
    let result = client
        .plan
        .execute(
            account_id,
            &ExecuteParams {
                plan_id: plan.plan_id.clone(),
                signature,
            },
        )
        .await?;

    let wait = args.get("wait").and_then(|v| v.as_bool()).unwrap_or(true);

    if wait {
        if let Some(activity_id) = &result.activity_id {
            let final_activity = poll_activity(client, account_id, activity_id).await?;
            return Ok(json!({
                "plan_id": result.plan_id,
                "activity_id": result.activity_id,
                "action": action,
                "status": final_activity.status,
                "activity": final_activity,
            })
            .to_string());
        }
    }

    // Not waiting or no activity_id — return execute result
    Ok(json!({
        "plan_id": result.plan_id,
        "activity_id": result.activity_id,
        "status": result.status,
        "action": action,
    })
    .to_string())
}

/// Sign an EIP-712 digest using the active profile's P256 key via Turnkey.
async fn sign_with_profile(env: Env, profile: &str, digest: &str) -> anyhow::Result<String> {
    let p = config::load_profile(env, profile).ok_or_else(|| {
        anyhow::anyhow!("No profile found. Use the login or create_account tool first.")
    })?;
    let signer = load_signer_from_profile(&p)?;
    let turnkey = TurnkeyClient::new(TurnkeyConfig {
        signer,
        sub_org_id: p.sub_org_id,
        ethereum_signer_address: p.ethereum_signer_address,
        api_base_url: None,
        verbose: false,
    });
    turnkey.sign_digest(digest).await.map_err(Into::into)
}

/// Poll an activity until it reaches a terminal state (completed or failed).
/// Emits progress to stderr so the user can see what's happening.
async fn poll_activity(
    client: &LegendPrime,
    account_id: &str,
    activity_id: &str,
) -> anyhow::Result<Activity> {
    let timeout = std::time::Instant::now() + std::time::Duration::from_secs(300);
    let mut polls = 0u32;

    eprintln!("[legend] Waiting for activity {activity_id}...");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        polls += 1;

        let activity = client
            .accounts
            .activity_by_id(account_id, activity_id)
            .await?;

        let status = activity.status.as_deref().unwrap_or("unknown");

        match status {
            "completed" => {
                eprintln!("[legend] Activity {activity_id} completed ({polls} polls, {}s)", polls * 2);
                return Ok(activity);
            }
            "failed" => {
                eprintln!("[legend] Activity {activity_id} failed ({polls} polls, {}s)", polls * 2);
                return Ok(activity);
            }
            _ if std::time::Instant::now() > timeout => {
                eprintln!("[legend] Activity {activity_id} timed out after 5 minutes (last status: {status})");
                anyhow::bail!(
                    "Timed out after 5 minutes waiting for activity {activity_id} (last status: {status}). Use get_activities to check later.",
                );
            }
            _ => {
                eprintln!("[legend] Activity {activity_id}: {status} (poll #{polls})");
            }
        }
    }
}

// --- Helpers ---

fn make_client(session: &McpSession) -> anyhow::Result<LegendPrime> {
    let query_key =
        resolve_query_key(&session.key, session.env, &session.profile).map_err(anyhow::Error::msg)?;
    let base_url = resolve_base_url(&None, session.env);
    Ok(LegendPrime::new(Config {
        query_key,
        base_url: Some(base_url),
        verbose: false,
    }))
}

/// Resolve account_id: explicit arg > session default > error.
fn resolve_account_id(args: &Value, session: &McpSession) -> anyhow::Result<String> {
    if let Some(id) = opt_str(args, "account_id") {
        return Ok(id);
    }
    if let Some(id) = &session.active_account_id {
        return Ok(id.clone());
    }
    anyhow::bail!(
        "No account_id provided and no active account set. Use set_account first, or pass account_id."
    )
}

/// Resolve an amount field, converting "max" to uint256.max.
fn resolve_amount(args: &Value, key: &str) -> anyhow::Result<String> {
    let raw = str_arg(args, key)?;
    if raw.eq_ignore_ascii_case("max") {
        Ok(UINT256_MAX.to_string())
    } else {
        Ok(raw)
    }
}

fn str_arg(args: &Value, key: &str) -> anyhow::Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: {key}"))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn bool_arg(args: &Value, key: &str) -> anyhow::Result<bool> {
    args.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| anyhow::anyhow!("Missing required boolean argument: {key}"))
}

fn u64_arg(args: &Value, key: &str) -> anyhow::Result<u64> {
    args.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing required integer argument: {key}"))
}

fn str_array_arg(args: &Value, key: &str) -> anyhow::Result<Vec<String>> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .ok_or_else(|| anyhow::anyhow!("Missing required array argument: {key}"))
}

fn generate_key(
    name: &str,
    use_file_key: bool,
    env: Env,
) -> anyhow::Result<(Box<dyn Signer>, String, Option<String>, Option<String>)> {
    if use_file_key {
        let dir = config::keys_dir(env);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}.key"));
        let signer = FileSigner::generate(&path)?;
        Ok((
            Box::new(signer),
            "file".into(),
            None,
            Some(path.to_string_lossy().to_string()),
        ))
    } else {
        #[cfg(feature = "keychain")]
        {
            let label = format!("com.legend.cli.{env}.{name}");
            let signer = KeychainSigner::generate(&label)?;
            Ok((Box::new(signer), "keychain".into(), Some(label), None))
        }
        #[cfg(not(feature = "keychain"))]
        {
            anyhow::bail!(
                "iCloud Keychain is not available in this build. Pass use_file_key: true,\n\
                 or install via `brew install legend-cli` for iCloud Keychain support."
            );
        }
    }
}

// --- JSON-RPC helpers ---

fn jsonrpc_result(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn jsonrpc_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
