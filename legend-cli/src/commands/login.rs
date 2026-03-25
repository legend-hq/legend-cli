use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::{self, Env};

/// Run the OAuth login flow: open browser → Google SSO → receive JWT → save to profile.
pub async fn login(base_url: &str, env: Env, profile_name: &str) -> anyhow::Result<()> {
    let prime_url = base_url.trim_end_matches('/');

    // 1. Generate PKCE verifier + challenge
    let verifier = generate_pkce_verifier();
    let challenge = pkce_challenge(&verifier);

    // 2. Start localhost listener on a random port
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| anyhow::anyhow!("Failed to bind: {e}"))?;
    let port = server.server_addr().to_ip().unwrap().port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    // 3. Generate state for CSRF
    let state = generate_state();

    // 4. Register as an OAuth client (RFC 7591 dynamic registration)
    let client_id = register_client(prime_url, &redirect_uri).await?;

    // 5. Build authorization URL and open browser
    let auth_url = format!(
        "{prime_url}/oauth/authorize/google?client_id={client_id}&response_type=code&redirect_uri={redirect_uri}&code_challenge={challenge}&code_challenge_method=S256&state={state}"
    );

    eprintln!("Opening browser for login...");
    if open::that(&auth_url).is_err() {
        eprintln!("Could not open browser. Please visit:\n{auth_url}");
    }

    // 6. Wait for the callback
    eprintln!("Waiting for authentication...");
    let (code, returned_state) = wait_for_callback(&server)?;

    if returned_state != state {
        anyhow::bail!("OAuth state mismatch — possible CSRF attack");
    }

    // 7. Exchange code for token
    let token_response =
        exchange_token(prime_url, &code, &verifier, &redirect_uri, &client_id).await?;

    let access_token = token_response["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No access_token in response"))?;

    // 8. Check if we need to select an account (multi-PA user)
    let token = if token_response.get("prime_account").is_some() {
        // Scoped token — ready to use
        let pa = &token_response["prime_account"];
        eprintln!(
            "Logged in as {} ({})",
            pa["name"].as_str().unwrap_or(""),
            pa["external_id"].as_str().unwrap_or("")
        );
        access_token.to_string()
    } else if let Some(accounts) = token_response.get("accounts") {
        // Unscoped token — need to select account
        let accounts = accounts
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Expected accounts array"))?;

        if accounts.is_empty() {
            anyhow::bail!("No Prime Accounts found for this user");
        }

        if accounts.len() == 1 {
            // Auto-select the only account
            let pa_id = accounts[0]["external_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing external_id"))?;

            let selected = select_account(prime_url, access_token, pa_id).await?;
            let token = selected["access_token"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("No access_token after account selection"))?;

            let pa = &selected["prime_account"];
            eprintln!(
                "Logged in as {} ({})",
                pa["name"].as_str().unwrap_or(""),
                pa["external_id"].as_str().unwrap_or("")
            );
            token.to_string()
        } else {
            // Multiple accounts — ask user to choose
            eprintln!("\nMultiple Prime Accounts found:");
            for (i, acct) in accounts.iter().enumerate() {
                eprintln!(
                    "  [{}] {} ({})",
                    i + 1,
                    acct["name"].as_str().unwrap_or("?"),
                    acct["external_id"].as_str().unwrap_or("?")
                );
            }
            eprint!("Select account [1]: ");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let choice: usize = input.trim().parse().unwrap_or(1);
            let idx = choice.saturating_sub(1).min(accounts.len() - 1);

            let pa_id = accounts[idx]["external_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing external_id"))?;

            let selected = select_account(prime_url, access_token, pa_id).await?;
            let token = selected["access_token"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("No access_token after account selection"))?;

            let pa = &selected["prime_account"];
            eprintln!(
                "Logged in as {} ({})",
                pa["name"].as_str().unwrap_or(""),
                pa["external_id"].as_str().unwrap_or("")
            );
            token.to_string()
        }
    } else {
        // Simple token response — just use the access_token
        access_token.to_string()
    };

    // 9. Save to profile
    let mut profile = config::load_profile(env, profile_name).unwrap_or_else(|| config::Profile {
        query_key: None,
        key_source: "none".into(),
        key_label: None,
        key_path: None,
        p256_public_key: String::new(),
        sub_org_id: String::new(),
        ethereum_signer_address: String::new(),
        account_external_id: String::new(),
    });
    profile.query_key = Some(token);
    config::save_profile(env, profile_name, &profile)?;

    eprintln!("Token saved to profile '{profile_name}'");
    Ok(())
}

// --- OAuth helpers ---

async fn register_client(prime_url: &str, redirect_uri: &str) -> anyhow::Result<String> {
    let http = reqwest::Client::new();
    let res = http
        .post(format!("{prime_url}/oauth/register"))
        .json(&serde_json::json!({
            "client_name": "Legend CLI",
            "redirect_uris": [redirect_uri],
            "grant_types": ["authorization_code"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none"
        }))
        .send()
        .await?;

    let body: serde_json::Value = res.json().await?;
    body["client_id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("No client_id in registration response: {body}"))
}

async fn exchange_token(
    prime_url: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
    client_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let http = reqwest::Client::new();
    let res = http
        .post(format!("{prime_url}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
        ])
        .send()
        .await?;

    if !res.status().is_success() {
        let body = res.text().await?;
        anyhow::bail!("Token exchange failed: {body}");
    }

    Ok(res.json().await?)
}

async fn select_account(
    prime_url: &str,
    unscoped_token: &str,
    pa_external_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let http = reqwest::Client::new();
    let res = http
        .post(format!("{prime_url}/dashboard/auth/select-account"))
        .header("Authorization", format!("Bearer {unscoped_token}"))
        .json(&serde_json::json!({
            "prime_account_id": pa_external_id
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let body = res.text().await?;
        anyhow::bail!("Account selection failed: {body}");
    }

    Ok(res.json().await?)
}

fn wait_for_callback(server: &tiny_http::Server) -> anyhow::Result<(String, String)> {
    // Wait up to 5 minutes for the callback
    let request = server
        .recv_timeout(std::time::Duration::from_secs(300))
        .map_err(|e| anyhow::anyhow!("Failed to receive callback: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("Timed out waiting for login callback"))?;

    let url = Url::parse(&format!("http://localhost{}", request.url()))?;
    let params: std::collections::HashMap<_, _> = url.query_pairs().collect();

    let code = params
        .get("code")
        .ok_or_else(|| {
            let error = params
                .get("error")
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".into());
            anyhow::anyhow!("Login failed: {error}")
        })?
        .to_string();

    let state = params
        .get("state")
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Send a nice response to the browser
    let response = tiny_http::Response::from_string(
        "<html><body><h1>Login successful!</h1><p>You can close this window.</p></body></html>",
    )
    .with_header(
        "Content-Type: text/html"
            .parse::<tiny_http::Header>()
            .unwrap(),
    );
    let _ = request.respond(response);

    Ok((code, state))
}

// --- PKCE ---

fn generate_pkce_verifier() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..16).map(|_| rng.r#gen()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}
