use legend_client::*;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(uri: &str) -> LegendPrime {
    LegendPrime::new(Config {
        query_key: "qk_test_secret".into(),
        base_url: Some(uri.to_string()),
        verbose: false,
    })
}

// --- Accounts ---

#[tokio::test]
async fn create_eoa_account() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/accounts"))
        .and(header("Authorization", "Bearer qk_test_secret"))
        .and(body_json(json!({
            "signer_type": "eoa",
            "ethereum_signer_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "account_id": "acc_test123",
            "signer_type": "eoa",
            "ethereum_signer_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18",
            "legend_wallet_address": "0xabc123",
            "created_at": "2026-03-18T00:00:00Z"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let account = client
        .accounts
        .create(&CreateAccountParams {
            signer_type: "eoa".into(),
            ethereum_signer_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(account.account_id, "acc_test123");
    assert_eq!(account.signer_type.as_deref(), Some("eoa"));
    assert_eq!(
        account.ethereum_signer_address.as_deref(),
        Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18")
    );
}

#[tokio::test]
async fn create_turnkey_p256_account() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/accounts"))
        .and(body_json(json!({
            "signer_type": "turnkey_p256",
            "p256_public_key": "0x02a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "account_id": "acc_tk256",
            "signer_type": "turnkey_p256",
            "ethereum_signer_address": "0xdeadbeef00000000000000000000000000000001",
            "legend_wallet_address": "0xabc456",
            "turnkey_sub_org_id": "sub_org_xyz",
            "created_at": "2026-03-18T00:00:00Z"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let account = client
        .accounts
        .create(&CreateAccountParams {
            signer_type: "turnkey_p256".into(),
            p256_public_key: Some(
                "0x02a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1".into(),
            ),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(account.account_id, "acc_tk256");
    assert_eq!(account.turnkey_sub_org_id.as_deref(), Some("sub_org_xyz"));
}

#[tokio::test]
async fn list_accounts() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accounts": [
                {
                    "account_id": "acc_1",
                    "signer_type": "eoa",
                    "ethereum_signer_address": "0xaaa",
                    "legend_wallet_address": "0xbbb",
                    "created_at": "2026-03-18T00:00:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let list = client.accounts.list().await.unwrap();

    assert_eq!(list.accounts.len(), 1);
    assert_eq!(list.accounts[0].account_id, "acc_1");
}

#[tokio::test]
async fn get_account() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/accounts/acc_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_id": "acc_123",
            "signer_type": "eoa",
            "ethereum_signer_address": "0xaaa",
            "legend_wallet_address": "0xbbb",
            "created_at": "2026-03-18T00:00:00Z"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let account = client.accounts.get("acc_123").await.unwrap();

    assert_eq!(account.account_id, "acc_123");
}

// --- Error handling ---

#[tokio::test]
async fn api_error_404() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/accounts/acc_nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "account_not_found",
            "detail": "Account not found"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.accounts.get("acc_nonexistent").await.unwrap_err();

    match err {
        LegendPrimeError::Api {
            code,
            message,
            status,
        } => {
            assert_eq!(code, "account_not_found");
            assert_eq!(message, "Account not found");
            assert_eq!(status, 404);
        }
        other => panic!("Expected API error, got: {other:?}"),
    }
}

#[tokio::test]
async fn api_error_400() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "code": "invalid_params",
            "detail": "Invalid signer_type"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client
        .accounts
        .create(&CreateAccountParams {
            signer_type: "bad".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();

    match err {
        LegendPrimeError::Api { code, status, .. } => {
            assert_eq!(code, "invalid_params");
            assert_eq!(status, 400);
        }
        other => panic!("Expected API error, got: {other:?}"),
    }
}

// --- Plan ---

#[tokio::test]
async fn plan_earn() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/accounts/acc_1/plan/earn"))
        .and(body_json(json!({
            "amount": "1000",
            "asset": "usdc",
            "network": "base",
            "protocol": "aave_v3"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "plan_id": "pln_abc",
            "details": {
                "eip712_data": {
                    "digest": "0xdeadbeef"
                }
            },
            "expires_at": "2026-03-18T01:00:00Z"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let plan = client
        .plan
        .earn(
            "acc_1",
            &EarnParams {
                amount: "1000".into(),
                asset: "usdc".into(),
                network: "base".into(),
                protocol: "aave_v3".into(),
                market: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(plan.plan_id, "pln_abc");
    assert_eq!(plan.digest(), Some("0xdeadbeef"));
}

#[tokio::test]
async fn plan_execute() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/accounts/acc_1/plan/execute"))
        .and(body_json(json!({
            "plan_id": "pln_abc",
            "signature": "0xdeadbeef"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "plan_id": "pln_abc",
            "quark_intent_id": "42",
            "status": "pending"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client
        .plan
        .execute(
            "acc_1",
            &ExecuteParams {
                plan_id: "pln_abc".into(),
                signature: "0xdeadbeef".into(),
            },
        )
        .await
        .unwrap();

    assert_eq!(result.plan_id, "pln_abc");
    assert_eq!(result.quark_intent_id.as_deref(), Some("42"));
    assert_eq!(result.status, "pending");
}

// --- Root methods ---

#[tokio::test]
async fn prime_account() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/prime_account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "pa_test",
            "name": "Test Corp",
            "email": "test@example.com"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let pa = client.prime_account().await.unwrap();

    assert_eq!(pa.id, "pa_test");
    assert_eq!(pa.name.as_deref(), Some("Test Corp"));
}

#[tokio::test]
async fn networks() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/networks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "networks": [
                { "name": "base", "chain_id": 8453, "display_name": "Base" },
                { "name": "ethereum", "chain_id": 1, "display_name": "Ethereum" }
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let networks = client.networks().await.unwrap();

    assert_eq!(networks.networks.len(), 2);
    assert_eq!(networks.networks[0].name, "base");
    assert_eq!(networks.networks[0].chain_id, 8453);
}

#[tokio::test]
async fn assets() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/assets"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "assets": {
                "usdc": {
                    "name": "usdc",
                    "decimals": 6,
                    "networks": {}
                }
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let assets = client.assets().await.unwrap();

    assert!(assets.assets["usdc"]["decimals"].as_u64() == Some(6));
}
