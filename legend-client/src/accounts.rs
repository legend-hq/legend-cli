use std::sync::Arc;

use reqwest::Method;

use crate::client::ClientInner;
use crate::error::Result;
use crate::types::*;

pub struct AccountsApi {
    pub(crate) inner: Arc<ClientInner>,
}

impl AccountsApi {
    pub async fn create(&self, params: &CreateAccountParams) -> Result<Account> {
        self.inner
            .request(Method::POST, "/accounts", Some(params))
            .await
    }

    pub async fn list(&self) -> Result<AccountList> {
        self.inner
            .request(Method::GET, "/accounts", None::<&()>)
            .await
    }

    pub async fn get(&self, external_id: &str) -> Result<Account> {
        self.inner
            .request(
                Method::GET,
                &format!("/accounts/{external_id}"),
                None::<&()>,
            )
            .await
    }

    pub async fn folio(&self, external_id: &str, opts: &FolioOpts) -> Result<Folio> {
        let qs = if opts.cached { "?cached=true" } else { "" };
        self.inner
            .request(
                Method::GET,
                &format!("/accounts/{external_id}/folio{qs}"),
                None::<&()>,
            )
            .await
    }

    pub async fn activities(&self, external_id: &str) -> Result<ActivityList> {
        self.inner
            .request(
                Method::GET,
                &format!("/accounts/{external_id}/activities"),
                None::<&()>,
            )
            .await
    }

    pub async fn activity(&self, external_id: &str, activity_id: u64) -> Result<Activity> {
        self.inner
            .request(
                Method::GET,
                &format!("/accounts/{external_id}/activities/{activity_id}"),
                None::<&()>,
            )
            .await
    }

    /// Get a single activity by its external ID (e.g. "act_xxx").
    pub async fn activity_by_id(&self, account_id: &str, activity_id: &str) -> Result<Activity> {
        self.inner
            .request(
                Method::GET,
                &format!("/accounts/{account_id}/activities/{activity_id}"),
                None::<&()>,
            )
            .await
    }

    pub async fn events(&self, external_id: &str, opts: &EventsOpts) -> Result<EventList> {
        let mut query_parts = vec![];
        if let Some(since) = opts.since {
            query_parts.push(format!("since={since}"));
        }
        if opts.poll {
            query_parts.push("poll=true".to_string());
        }
        let qs = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };
        self.inner
            .request(
                Method::GET,
                &format!("/accounts/{external_id}/events{qs}"),
                None::<&()>,
            )
            .await
    }
}
