use serde::{Deserialize, Serialize};

// --- Config ---

pub struct Config {
    pub query_key: String,
    pub base_url: Option<String>,
    pub verbose: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FolioOpts {
    pub cached: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EventsOpts {
    pub since: Option<u64>,
    pub poll: bool,
}

// --- Account types ---

#[derive(Debug, Serialize, Default)]
pub struct CreateAccountParams {
    pub signer_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethereum_signer_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solana_signer_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p256_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_storage: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub account_id: String,
    pub signer_type: Option<String>,
    pub ethereum_signer_address: Option<String>,
    pub p256_public_key: Option<String>,
    pub legend_wallet_address: Option<String>,
    pub solana_wallet_address: Option<String>,
    pub turnkey_sub_org_id: Option<String>,
    pub key_storage: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountList {
    pub accounts: Vec<Account>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrimeAccount {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Folio {
    pub folio: serde_json::Value,
}

// --- Plan types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub plan_id: String,
    pub details: serde_json::Value,
    pub expires_at: String,
}

impl Plan {
    /// Extract the EIP-712 digest from plan details.
    pub fn digest(&self) -> Option<&str> {
        self.details
            .get("eip712_data")
            .and_then(|d| d.get("digest"))
            .and_then(|d| d.as_str())
    }
}

#[derive(Debug, Serialize)]
pub struct ExecuteParams {
    pub plan_id: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub plan_id: String,
    pub quark_intent_id: Option<String>,
    pub activity_id: Option<String>,
    pub status: String,
}

// --- Plan request params ---

#[derive(Debug, Serialize)]
pub struct EarnParams {
    pub amount: String,
    pub asset: String,
    pub network: String,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WithdrawParams {
    pub amount: String,
    pub asset: String,
    pub network: String,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransferParams {
    pub amount: String,
    pub asset: String,
    pub network: String,
    pub recipient: String,
}

#[derive(Debug, Serialize)]
pub struct ClaimRewardsParams {
    pub asset: String,
}

#[derive(Debug, Serialize)]
pub struct BorrowParams {
    pub amount: String,
    pub asset: String,
    pub network: String,
    pub collateral_amount: String,
    pub collateral_asset: String,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepayParams {
    pub amount: String,
    pub asset: String,
    pub network: String,
    pub collateral_amount: String,
    pub collateral_asset: String,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SwapParams {
    pub sell_asset: String,
    pub buy_asset: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sell_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buy_amount: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoopLongParams {
    pub exposure_asset: String,
    pub backing_asset: String,
    pub market_id: String,
    pub is_increase: bool,
    pub exposure_amount: String,
    pub max_swap_backing_amount: String,
    pub max_provided_backing_amount: String,
    pub pool_fee: u64,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct UnloopLongParams {
    pub exposure_asset: String,
    pub backing_asset: String,
    pub market_id: String,
    pub exposure_amount: String,
    pub backing_amount_to_exit: String,
    pub min_swap_backing_amount: String,
    pub pool_fee: u64,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct AddBackingParams {
    pub exposure_asset: String,
    pub backing_asset: String,
    pub market_id: String,
    pub amount: String,
    pub is_short: bool,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct WithdrawBackingParams {
    pub exposure_asset: String,
    pub backing_asset: String,
    pub market_id: String,
    pub amount: String,
    pub is_short: bool,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct MigrateParams {
    pub amount: String,
    pub asset: String,
    pub from_protocol: String,
    pub to_protocol: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_market: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_market: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_only_supply_balances: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SwapAndSupplyParams {
    pub sell_asset: String,
    pub sell_amount: String,
    pub buy_asset: String,
    pub protocol: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReinvestRewardsParams {
    pub asset: String,
    pub protocol: String,
    pub network: String,
    pub reward_assets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

// --- Activity types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityList {
    pub activities: Vec<Activity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Activity {
    pub id: u64,
    pub status: Option<String>,
    pub quark_intent: Option<serde_json::Value>,
    pub executions: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventList {
    pub events: Vec<serde_json::Value>,
    pub cursor: Option<u64>,
}

// --- Reference types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkList {
    pub networks: Vec<Network>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    pub name: String,
    pub chain_id: u64,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetMap {
    pub assets: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketList {
    pub markets: Vec<Market>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "protocol")]
pub enum Market {
    #[serde(rename = "morpho_market")]
    MorphoMarket {
        chain_id: u64,
        morpho: String,
        market_id: String,
        irm: String,
        lltv: u64,
        oracle: String,
        loan_token: String,
        collateral_token: String,
        wad: u64,
    },
    #[serde(rename = "morpho_vault")]
    MorphoVault {
        chain_id: u64,
        name: String,
        symbol: String,
        vault: String,
        asset: String,
        wad: u64,
    },
    #[serde(rename = "aave_market")]
    AaveMarket {
        chain_id: u64,
        name: String,
        pool: String,
        ui_pool_data_provider: String,
        market_base_currency: String,
        ray_scale: f64,
        bps_scale: f64,
        reserves: Vec<AaveReserve>,
    },
    #[serde(rename = "comet")]
    Comet {
        chain_id: u64,
        name: String,
        symbol: String,
        base_asset: String,
        factor_scale: u64,
        comet_address: String,
        rewards_address: String,
        collateral_assets: Vec<CometCollateral>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AaveReserve {
    pub symbol: String,
    pub decimals: u64,
    pub underlying_asset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CometCollateral {
    pub asset: String,
    pub price_feed: String,
}
