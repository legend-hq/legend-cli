use std::sync::Arc;

use reqwest::Method;

use crate::client::ClientInner;
use crate::error::Result;
use crate::types::*;

pub struct PlanApi {
    pub(crate) inner: Arc<ClientInner>,
}

impl PlanApi {
    pub async fn earn(&self, account_id: &str, params: &EarnParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/earn"),
                Some(params),
            )
            .await
    }

    pub async fn withdraw(&self, account_id: &str, params: &WithdrawParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/withdraw"),
                Some(params),
            )
            .await
    }

    pub async fn transfer(&self, account_id: &str, params: &TransferParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/transfer"),
                Some(params),
            )
            .await
    }

    pub async fn claim_rewards(
        &self,
        account_id: &str,
        params: &ClaimRewardsParams,
    ) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/claim-rewards"),
                Some(params),
            )
            .await
    }

    pub async fn borrow(&self, account_id: &str, params: &BorrowParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/borrow"),
                Some(params),
            )
            .await
    }

    pub async fn repay(&self, account_id: &str, params: &RepayParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/repay"),
                Some(params),
            )
            .await
    }

    pub async fn swap(&self, account_id: &str, params: &SwapParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/swap"),
                Some(params),
            )
            .await
    }

    pub async fn loop_long(&self, account_id: &str, params: &LoopLongParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/loop-long"),
                Some(params),
            )
            .await
    }

    pub async fn unloop_long(&self, account_id: &str, params: &UnloopLongParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/unloop-long"),
                Some(params),
            )
            .await
    }

    pub async fn add_backing(&self, account_id: &str, params: &AddBackingParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/add-backing"),
                Some(params),
            )
            .await
    }

    pub async fn withdraw_backing(
        &self,
        account_id: &str,
        params: &WithdrawBackingParams,
    ) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/withdraw-backing"),
                Some(params),
            )
            .await
    }

    pub async fn migrate(&self, account_id: &str, params: &MigrateParams) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/migrate"),
                Some(params),
            )
            .await
    }

    pub async fn swap_and_supply(
        &self,
        account_id: &str,
        params: &SwapAndSupplyParams,
    ) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/swap-and-supply"),
                Some(params),
            )
            .await
    }

    pub async fn reinvest_rewards(
        &self,
        account_id: &str,
        params: &ReinvestRewardsParams,
    ) -> Result<Plan> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/reinvest_rewards"),
                Some(params),
            )
            .await
    }

    pub async fn execute(&self, account_id: &str, params: &ExecuteParams) -> Result<ExecuteResult> {
        self.inner
            .request(
                Method::POST,
                &format!("/accounts/{account_id}/plan/execute"),
                Some(params),
            )
            .await
    }
}
