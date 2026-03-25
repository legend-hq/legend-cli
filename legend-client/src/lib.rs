pub mod accounts;
pub mod client;
pub mod error;
pub mod plan;
pub mod types;

pub use client::LegendPrime;
pub use error::{LegendPrimeError, Result};
pub use types::*;

use reqwest::Method;

impl LegendPrime {
    pub async fn prime_account(&self) -> Result<PrimeAccount> {
        self.inner
            .request(Method::GET, "/prime_account", None::<&()>)
            .await
    }

    pub async fn networks(&self) -> Result<NetworkList> {
        self.inner
            .request(Method::GET, "/networks", None::<&()>)
            .await
    }

    pub async fn assets(&self) -> Result<AssetMap> {
        self.inner
            .request(Method::GET, "/assets", None::<&()>)
            .await
    }
}
