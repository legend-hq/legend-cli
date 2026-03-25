use legend_client::*;
use legend_signer::*;

use crate::auth::resolve_query_key;
use crate::commands::sign::load_signer_from_profile;
use crate::config::{Env, load_profile};
use crate::output::*;

/// Helper: if --execute is set, sign the plan digest and execute it.
/// Otherwise, print the plan.
async fn maybe_execute(
    client: &LegendPrime,
    plan: &Plan,
    account_id: &str,
    execute: bool,
    env: Env,
    profile_name: &str,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    if execute {
        let digest = plan
            .digest()
            .ok_or_else(|| anyhow::anyhow!("Plan response missing digest"))?;

        let profile = load_profile(env, profile_name).ok_or_else(|| {
            anyhow::anyhow!("No profile found. Run: legend-cli accounts create --keygen")
        })?;

        let signer = load_signer_from_profile(&profile)?;
        let turnkey = TurnkeyClient::new(TurnkeyConfig {
            signer,
            sub_org_id: profile.sub_org_id.clone(),
            ethereum_signer_address: profile.ethereum_signer_address.clone(),
            api_base_url: None,
            verbose,
        });

        let signature = turnkey.sign_digest(digest).await?;
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

        print_execute_result(&result, mode);
    } else {
        print_plan(plan, mode);
    }
    Ok(())
}

pub async fn earn(
    account_id: &str,
    amount: &str,
    asset: &str,
    network: &str,
    protocol: &str,
    market: &Option<String>,
    execute: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let plan = client
        .plan
        .earn(
            account_id,
            &EarnParams {
                amount: amount.into(),
                asset: asset.into(),
                network: network.into(),
                protocol: protocol.into(),
                market: market.clone(),
            },
        )
        .await?;

    maybe_execute(
        &client,
        &plan,
        account_id,
        execute,
        env,
        profile_name,
        verbose,
        mode,
    )
    .await
}

pub async fn swap(
    account_id: &str,
    sell_asset: &str,
    buy_asset: &str,
    sell_amount: &Option<String>,
    buy_amount: &Option<String>,
    network: &str,
    execute: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let plan = client
        .plan
        .swap(
            account_id,
            &SwapParams {
                sell_asset: sell_asset.into(),
                buy_asset: buy_asset.into(),
                network: network.into(),
                sell_amount: sell_amount.clone(),
                buy_amount: buy_amount.clone(),
            },
        )
        .await?;

    maybe_execute(
        &client,
        &plan,
        account_id,
        execute,
        env,
        profile_name,
        verbose,
        mode,
    )
    .await
}

pub async fn borrow(
    account_id: &str,
    amount: &str,
    asset: &str,
    collateral_amount: &str,
    collateral_asset: &str,
    network: &str,
    protocol: &str,
    market: &Option<String>,
    execute: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let plan = client
        .plan
        .borrow(
            account_id,
            &BorrowParams {
                amount: amount.into(),
                asset: asset.into(),
                network: network.into(),
                collateral_amount: collateral_amount.into(),
                collateral_asset: collateral_asset.into(),
                protocol: protocol.into(),
                market: market.clone(),
            },
        )
        .await?;

    maybe_execute(
        &client,
        &plan,
        account_id,
        execute,
        env,
        profile_name,
        verbose,
        mode,
    )
    .await
}

pub async fn withdraw(
    account_id: &str,
    amount: &str,
    asset: &str,
    network: &str,
    protocol: &str,
    market: &Option<String>,
    execute: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let plan = client
        .plan
        .withdraw(
            account_id,
            &WithdrawParams {
                amount: amount.into(),
                asset: asset.into(),
                network: network.into(),
                protocol: protocol.into(),
                market: market.clone(),
            },
        )
        .await?;

    maybe_execute(
        &client,
        &plan,
        account_id,
        execute,
        env,
        profile_name,
        verbose,
        mode,
    )
    .await
}

pub async fn transfer(
    account_id: &str,
    amount: &str,
    asset: &str,
    network: &str,
    recipient: &str,
    execute: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let plan = client
        .plan
        .transfer(
            account_id,
            &TransferParams {
                amount: amount.into(),
                asset: asset.into(),
                network: network.into(),
                recipient: recipient.into(),
            },
        )
        .await?;

    maybe_execute(
        &client,
        &plan,
        account_id,
        execute,
        env,
        profile_name,
        verbose,
        mode,
    )
    .await
}

pub async fn repay(
    account_id: &str,
    amount: &str,
    asset: &str,
    collateral_amount: &str,
    collateral_asset: &str,
    network: &str,
    protocol: &str,
    market: &Option<String>,
    execute: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let plan = client
        .plan
        .repay(
            account_id,
            &RepayParams {
                amount: amount.into(),
                asset: asset.into(),
                network: network.into(),
                collateral_amount: collateral_amount.into(),
                collateral_asset: collateral_asset.into(),
                protocol: protocol.into(),
                market: market.clone(),
            },
        )
        .await?;

    maybe_execute(
        &client,
        &plan,
        account_id,
        execute,
        env,
        profile_name,
        verbose,
        mode,
    )
    .await
}

pub async fn execute_plan(
    account_id: &str,
    plan_id: &str,
    auto_sign: bool,
    digest: &Option<String>,
    signature: &Option<String>,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let sig = if auto_sign {
        let digest_val = digest
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--auto-sign requires --digest <0x...>"))?;

        let profile = load_profile(env, profile_name).ok_or_else(|| {
            anyhow::anyhow!("No profile found. Run: legend-cli accounts create --keygen")
        })?;

        let signer = load_signer_from_profile(&profile)?;
        let turnkey = TurnkeyClient::new(TurnkeyConfig {
            signer,
            sub_org_id: profile.sub_org_id,
            ethereum_signer_address: profile.ethereum_signer_address,
            api_base_url: None,
            verbose,
        });

        turnkey.sign_digest(digest_val).await?
    } else {
        signature
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Either --auto-sign --digest or --signature required"))?
    };

    let result = client
        .plan
        .execute(
            account_id,
            &ExecuteParams {
                plan_id: plan_id.to_string(),
                signature: sig,
            },
        )
        .await?;

    print_execute_result(&result, mode);
    Ok(())
}
