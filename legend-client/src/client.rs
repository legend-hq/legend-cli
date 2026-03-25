use std::sync::Arc;

use reqwest::{Client, Method};
use serde::{Serialize, de::DeserializeOwned};

use crate::accounts::AccountsApi;
use crate::error::{LegendPrimeError, Result};
use crate::plan::PlanApi;
use crate::types::Config;

const DEFAULT_BASE_URL: &str = "https://prime-api.legend.xyz";

pub(crate) struct ClientInner {
    http: Client,
    base_url: String,
    query_key: String,
    verbose: bool,
}

impl ClientInner {
    pub(crate) async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + Sync)>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        if self.verbose {
            eprintln!("[verbose] {} {}", method, url);
            if let Some(body) = &body {
                if let Ok(json) = serde_json::to_string(body) {
                    eprintln!("[verbose] body: {}", json);
                }
            }
        }

        let mut req = self
            .http
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.query_key))
            .header("Accept", "application/json");

        if let Some(body) = body {
            req = req.json(body);
        }

        let res = req.send().await.map_err(LegendPrimeError::Http)?;
        let status = res.status();

        if self.verbose {
            eprintln!("[verbose] response: {}", status);
        }

        if !status.is_success() {
            let raw_body = res.text().await.unwrap_or_default();

            if self.verbose {
                eprintln!("[verbose] error body: {}", raw_body);
            }

            let err_body: serde_json::Value = serde_json::from_str(&raw_body)
                .unwrap_or_else(|_| serde_json::json!({"code": "unknown", "detail": raw_body}));

            return Err(LegendPrimeError::Api {
                code: err_body["code"].as_str().unwrap_or("unknown").to_string(),
                message: err_body["detail"]
                    .as_str()
                    .unwrap_or(&raw_body)
                    .to_string(),
                status: status.as_u16(),
            });
        }

        let raw_body = res.text().await.map_err(LegendPrimeError::Http)?;

        if self.verbose {
            eprintln!(
                "[verbose] response body: {}",
                &raw_body[..raw_body.len().min(500)]
            );
        }

        serde_json::from_str(&raw_body).map_err(LegendPrimeError::Deserialize)
    }
}

pub struct LegendPrime {
    pub(crate) inner: Arc<ClientInner>,
    pub accounts: AccountsApi,
    pub plan: PlanApi,
}

impl LegendPrime {
    pub fn new(config: Config) -> Self {
        let base_url = config
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let http = Client::new();
        let inner = Arc::new(ClientInner {
            http,
            base_url,
            query_key: config.query_key,
            verbose: config.verbose,
        });

        Self {
            accounts: AccountsApi {
                inner: inner.clone(),
            },
            plan: PlanApi {
                inner: inner.clone(),
            },
            inner,
        }
    }
}
