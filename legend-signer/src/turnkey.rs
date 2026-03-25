use base64::Engine;
use sha3::{Digest, Keccak256};

use crate::error::{Result, SignerError};
use crate::signer::Signer;

pub struct TurnkeyClient {
    signer: Box<dyn Signer>,
    sub_org_id: String,
    ethereum_signer_address: String,
    api_base_url: String,
    verbose: bool,
    http: reqwest::Client,
}

pub struct TurnkeyConfig {
    pub signer: Box<dyn Signer>,
    pub sub_org_id: String,
    pub ethereum_signer_address: String,
    pub api_base_url: Option<String>,
    pub verbose: bool,
}

const DEFAULT_TURNKEY_URL: &str = "https://api.turnkey.com";

impl TurnkeyClient {
    pub fn new(config: TurnkeyConfig) -> Self {
        Self {
            signer: config.signer,
            sub_org_id: config.sub_org_id,
            ethereum_signer_address: config.ethereum_signer_address,
            api_base_url: config
                .api_base_url
                .unwrap_or_else(|| DEFAULT_TURNKEY_URL.to_string()),
            verbose: config.verbose,
            http: reqwest::Client::new(),
        }
    }

    /// Create a Turnkey API stamp for the given JSON body string.
    ///
    /// Protocol:
    /// 1. Sign the body with ECDSA-P256-SHA256 (DER-encoded)
    /// 2. Build stamp JSON: { publicKey (hex, no 0x), signature (uppercase hex DER), scheme }
    /// 3. Base64URL-encode (no padding)
    pub fn stamp(&self, body: &str) -> Result<String> {
        let signature_der = self.signer.sign(body.as_bytes())?;

        let pubkey = self.signer.public_key_hex();
        let pubkey_stripped = pubkey.strip_prefix("0x").unwrap_or(pubkey);

        let stamp = serde_json::json!({
            "publicKey": pubkey_stripped,
            "signature": hex::encode_upper(&signature_der),
            "scheme": "SIGNATURE_SCHEME_TK_API_P256"
        });

        let stamp_json = serde_json::to_string(&stamp)?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&stamp_json))
    }

    /// Query Turnkey's API with a stamped POST request.
    pub async fn query(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let body_str = serde_json::to_string(&body)?;
        let stamp = self.stamp(&body_str)?;

        let url = format!("{}{}", self.api_base_url, path);

        if self.verbose {
            eprintln!("[verbose] POST {url}");
            eprintln!("[verbose] body: {body_str}");
        }

        let res = self
            .http
            .post(&url)
            .header("X-Stamp", &stamp)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(SignerError::Http)?;

        let status = res.status();
        let raw_body = res.text().await.map_err(SignerError::Http)?;

        if self.verbose {
            eprintln!("[verbose] response: {status}");
            eprintln!("[verbose] response body: {raw_body}");
        }

        if !status.is_success() {
            return Err(SignerError::Turnkey(format!(
                "Turnkey API returned {status}: {raw_body}"
            )));
        }

        Ok(serde_json::from_str(&raw_body)?)
    }

    /// List wallets in the sub-organization.
    pub async fn list_wallets(&self) -> Result<serde_json::Value> {
        self.query(
            "/public/v1/query/list_wallets",
            serde_json::json!({
                "organizationId": self.sub_org_id,
            }),
        )
        .await
    }

    /// List accounts (addresses) for a specific wallet.
    pub async fn list_wallet_accounts(&self, wallet_id: &str) -> Result<serde_json::Value> {
        self.query(
            "/public/v1/query/list_wallet_accounts",
            serde_json::json!({
                "organizationId": self.sub_org_id,
                "walletId": wallet_id,
            }),
        )
        .await
    }

    /// Sign an EIP-712 digest via Turnkey's sign_raw_payload API.
    ///
    /// The digest should be a 0x-prefixed hex string (the EIP-712 hash).
    /// Returns a 0x-prefixed hex signature: `0x{r}{s}{v}` (132 chars).
    pub async fn sign_digest(&self, digest: &str) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
            .to_string();

        let body = serde_json::json!({
            "type": "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2",
            "timestampMs": timestamp,
            "organizationId": self.sub_org_id,
            "parameters": {
                "signWith": eip55_checksum(&self.ethereum_signer_address),
                "payload": digest,
                "encoding": "PAYLOAD_ENCODING_HEXADECIMAL",
                "hashFunction": "HASH_FUNCTION_NO_OP"
            }
        });

        let body_str = serde_json::to_string(&body)?;
        let stamp = self.stamp(&body_str)?;

        let url = format!("{}/public/v1/submit/sign_raw_payload", self.api_base_url);

        if self.verbose {
            eprintln!("[verbose] POST {url}");
            eprintln!("[verbose] body: {body_str}");
        }

        let res = self
            .http
            .post(&url)
            .header("X-Stamp", &stamp)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(SignerError::Http)?;

        let status = res.status();
        let raw_body = res.text().await.map_err(SignerError::Http)?;

        if self.verbose {
            eprintln!("[verbose] response: {status}");
            eprintln!(
                "[verbose] response body: {}",
                &raw_body[..raw_body.len().min(500)]
            );
        }

        if !status.is_success() {
            return Err(SignerError::Turnkey(format!(
                "Turnkey API returned {status}: {raw_body}"
            )));
        }

        let activity: serde_json::Value = serde_json::from_str(&raw_body)?;
        let result = self.resolve_activity(activity).await?;
        self.extract_signature(&result)
    }

    /// Handle Turnkey's async activity model — resolve immediately or poll.
    async fn resolve_activity(&self, activity: serde_json::Value) -> Result<serde_json::Value> {
        let status = activity["activity"]["status"].as_str().unwrap_or("");

        match status {
            "ACTIVITY_STATUS_COMPLETED" => {
                Ok(activity["activity"]["result"]["signRawPayloadResult"].clone())
            }
            "ACTIVITY_STATUS_CREATED" | "ACTIVITY_STATUS_PENDING" => {
                let activity_id = activity["activity"]["id"]
                    .as_str()
                    .ok_or_else(|| SignerError::Turnkey("Missing activity ID".into()))?;
                let org_id = activity["activity"]["organizationId"]
                    .as_str()
                    .unwrap_or(&self.sub_org_id);

                self.poll_activity(activity_id, org_id).await
            }
            _ => Err(SignerError::Turnkey(format!(
                "Unexpected activity status: {status}"
            ))),
        }
    }

    /// Poll a Turnkey activity by ID until it reaches a terminal state.
    async fn poll_activity(&self, activity_id: &str, org_id: &str) -> Result<serde_json::Value> {
        let url = format!("{}/public/v1/query/get_activity", self.api_base_url);

        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let body = serde_json::json!({
                "organizationId": org_id,
                "activityId": activity_id
            });
            let body_str = serde_json::to_string(&body)?;
            let stamp = self.stamp(&body_str)?;

            let res = self
                .http
                .post(&url)
                .header("X-Stamp", &stamp)
                .header("Content-Type", "application/json")
                .body(body_str)
                .send()
                .await
                .map_err(SignerError::Http)?;

            let activity: serde_json::Value = res.json().await.map_err(SignerError::Http)?;
            let status = activity["activity"]["status"].as_str().unwrap_or("");

            match status {
                "ACTIVITY_STATUS_COMPLETED" => {
                    return Ok(activity["activity"]["result"]["signRawPayloadResult"].clone());
                }
                "ACTIVITY_STATUS_FAILED" | "ACTIVITY_STATUS_REJECTED" => {
                    return Err(SignerError::Turnkey(format!(
                        "Activity {activity_id} failed: {status}"
                    )));
                }
                _ => continue,
            }
        }

        Err(SignerError::Turnkey(format!(
            "Activity {activity_id} timed out after 15s"
        )))
    }

    /// Extract {r, s, v} from Turnkey's signRawPayloadResult.
    /// Returns `0x{r}{s}{v}` (132 hex chars).
    fn extract_signature(&self, result: &serde_json::Value) -> Result<String> {
        let r = result["r"]
            .as_str()
            .ok_or_else(|| SignerError::Turnkey("Missing r in sign result".into()))?;
        let s = result["s"]
            .as_str()
            .ok_or_else(|| SignerError::Turnkey("Missing s in sign result".into()))?;
        let v_raw = result["v"]
            .as_str()
            .ok_or_else(|| SignerError::Turnkey("Missing v in sign result".into()))?;

        // Turnkey returns v as "00" or "01". EIP-155 expects 27 or 28.
        let v_int = u8::from_str_radix(v_raw, 16)
            .map_err(|_| SignerError::Turnkey(format!("Invalid v value: {v_raw}")))?;
        let v = if v_int < 27 { v_int + 27 } else { v_int };

        Ok(format!(
            "0x{}{}{}",
            r.to_lowercase(),
            s.to_lowercase(),
            format!("{v:02x}")
        ))
    }
}

/// EIP-55 checksum an Ethereum address.
/// Takes "0x064c..." (any case) and returns "0x064C538770614AA59A0a7c06A964141dDFf7e0aA".
fn eip55_checksum(address: &str) -> String {
    let addr = address.strip_prefix("0x").unwrap_or(address).to_lowercase();
    let hash = Keccak256::digest(addr.as_bytes());

    let checksummed: String = addr
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if c.is_ascii_alphabetic() {
                // Each hex char of the hash controls the case of the corresponding address char
                let hash_nibble = if i % 2 == 0 {
                    hash[i / 2] >> 4
                } else {
                    hash[i / 2] & 0x0f
                };
                if hash_nibble >= 8 {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            } else {
                c
            }
        })
        .collect();

    format!("0x{checksummed}")
}
