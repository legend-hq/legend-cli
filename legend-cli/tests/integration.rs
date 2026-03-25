#![cfg(feature = "integration")]
//! Integration tests for the Rust Prime API client.
//!
//! These tests run against a real Legend server + Turnkey API, orchestrated by
//! Elixir's PrimeClientCase. Config arrives via LEGEND_TEST_CONFIG env var.
//!
//! The tests generate their own P256 keys (via FileSigner), create Turnkey-backed
//! accounts, and self-fund via the funding server (POST /fund) provided by Elixir.

use legend_client::*;
use legend_signer::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestConfig {
    prime_api_url: String,
    query_key: String,
    #[serde(default)]
    funding_url: Option<String>,
    #[serde(default)]
    turnkey_api_url: Option<String>,
}

fn load_test_config() -> TestConfig {
    let json = std::env::var("LEGEND_TEST_CONFIG").expect("LEGEND_TEST_CONFIG not set");
    serde_json::from_str(&json).expect("Invalid LEGEND_TEST_CONFIG JSON")
}

fn make_client(config: &TestConfig) -> LegendPrime {
    LegendPrime::new(Config {
        query_key: config.query_key.clone(),
        base_url: Some(config.prime_api_url.clone()),
        verbose: false,
    })
}

fn make_file_signer() -> (FileSigner, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.key");
    let signer = FileSigner::generate(&path).unwrap();
    (signer, dir)
}

/// Call the Elixir funding server to fund a quark wallet and checkpoint it.
/// By the time this returns 200, the folio reflects the new balance.
async fn fund(config: &TestConfig, signer_address: &str, asset: &str, amount: u64, network: &str) {
    let funding_url = config
        .funding_url
        .as_ref()
        .expect("fundingUrl not in test config");

    let http = reqwest::Client::new();
    let res = http
        .post(format!("{funding_url}/fund"))
        .json(&serde_json::json!({
            "signer_address": signer_address,
            "asset": asset,
            "amount": amount,
            "network": network
        }))
        .send()
        .await
        .expect("Failed to call funding server");

    assert!(
        res.status().is_success(),
        "Funding server returned {}",
        res.status()
    );
}

// --- Account tests ---

#[tokio::test]
async fn test_account_create_and_query() {
    let config = load_test_config();
    let client = make_client(&config);
    let (signer, _dir) = make_file_signer();

    // Create turnkey_p256 account
    let account = client
        .accounts
        .create(&CreateAccountParams {
            signer_type: "turnkey_p256".into(),
            p256_public_key: Some(signer.public_key_hex().to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert!(account.account_id.starts_with("acc_"));
    assert_eq!(account.signer_type.as_deref(), Some("turnkey_p256"));
    assert!(account.ethereum_signer_address.is_some());
    assert!(account.legend_wallet_address.is_some());
    assert!(account.turnkey_sub_org_id.is_some());

    // Get by ID
    let fetched = client.accounts.get(&account.account_id).await.unwrap();
    assert_eq!(fetched.account_id, account.account_id);
    assert_eq!(
        fetched.ethereum_signer_address,
        account.ethereum_signer_address
    );

    // List
    let list = client.accounts.list().await.unwrap();
    assert!(
        list.accounts
            .iter()
            .any(|a| a.account_id == account.account_id)
    );
}

#[tokio::test]
async fn test_reference_data() {
    let config = load_test_config();
    let client = make_client(&config);

    let networks = client.networks().await.unwrap();
    assert!(!networks.networks.is_empty());

    let assets = client.assets().await.unwrap();
    assert!(!assets.assets.as_object().unwrap().is_empty());

    let prime = client.prime_account().await.unwrap();
    assert!(!prime.id.is_empty());
}

// --- Swap test (full flow: keygen → create → fund → plan → sign → execute) ---

#[tokio::test]
async fn test_swap_full_flow() {
    let config = load_test_config();
    let client = make_client(&config);
    let (signer, _dir) = make_file_signer();

    // 1. Create turnkey_p256 account
    let account = client
        .accounts
        .create(&CreateAccountParams {
            signer_type: "turnkey_p256".into(),
            p256_public_key: Some(signer.public_key_hex().to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    let signer_address = account.ethereum_signer_address.as_ref().unwrap();
    let sub_org_id = account.turnkey_sub_org_id.as_ref().unwrap();

    // 2. Fund the quark wallet (Elixir handles set_balance + checkpoint)
    fund(&config, signer_address, "usdc", 10_000_000, "base").await;

    // 3. Create swap plan
    let plan = client
        .plan
        .swap(
            &account.account_id,
            &SwapParams {
                sell_asset: "USDC".into(),
                buy_asset: "WETH".into(),
                sell_amount: Some("1000000".into()),
                buy_amount: None,
                network: "base".into(),
            },
        )
        .await
        .unwrap();

    assert!(!plan.plan_id.is_empty());
    let digest = plan.digest().expect("Plan missing digest");

    // 4. Sign via Turnkey
    let turnkey = TurnkeyClient::new(TurnkeyConfig {
        signer: Box::new(signer),
        sub_org_id: sub_org_id.clone(),
        ethereum_signer_address: signer_address.clone(),
        api_base_url: config.turnkey_api_url.clone(),
        verbose: false,
    });

    let signature = turnkey.sign_digest(digest).await.unwrap();
    assert!(signature.starts_with("0x"));
    assert_eq!(signature.len(), 132);

    // 5. Execute
    let result = client
        .plan
        .execute(
            &account.account_id,
            &ExecuteParams {
                plan_id: plan.plan_id.clone(),
                signature,
            },
        )
        .await
        .unwrap();

    assert!(!result.plan_id.is_empty());
    assert!(!result.status.is_empty());
}

// --- Earn test (full flow) ---

#[tokio::test]
async fn test_earn_full_flow() {
    let config = load_test_config();
    let client = make_client(&config);
    let (signer, _dir) = make_file_signer();

    // 1. Create account
    let account = client
        .accounts
        .create(&CreateAccountParams {
            signer_type: "turnkey_p256".into(),
            p256_public_key: Some(signer.public_key_hex().to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    let signer_address = account.ethereum_signer_address.as_ref().unwrap();
    let sub_org_id = account.turnkey_sub_org_id.as_ref().unwrap();

    // 2. Fund
    fund(&config, signer_address, "usdc", 10_000_000, "base").await;

    // 3. Create earn plan
    let plan = client
        .plan
        .earn(
            &account.account_id,
            &EarnParams {
                amount: "1000000".into(),
                asset: "USDC".into(),
                network: "base".into(),
                protocol: "compound".into(),
                market: None,
            },
        )
        .await
        .unwrap();

    let digest = plan.digest().expect("Plan missing digest");

    // 4. Sign
    let turnkey = TurnkeyClient::new(TurnkeyConfig {
        signer: Box::new(signer),
        sub_org_id: sub_org_id.clone(),
        ethereum_signer_address: signer_address.clone(),
        api_base_url: config.turnkey_api_url.clone(),
        verbose: false,
    });

    let signature = turnkey.sign_digest(digest).await.unwrap();

    // 5. Execute
    let result = client
        .plan
        .execute(
            &account.account_id,
            &ExecuteParams {
                plan_id: plan.plan_id.clone(),
                signature,
            },
        )
        .await
        .unwrap();

    assert!(!result.plan_id.is_empty());
    assert!(!result.status.is_empty());
}
