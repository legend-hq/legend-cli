# legend-client

Rust client library for the [Legend](https://legend.xyz) API. This is the same client that powers `legend-cli`.

## Install

```toml
[dependencies]
legend-client = "0.0.1"
tokio = { version = "1", features = ["full"] }
```

## Usage

```rust
use legend_client::{LegendPrime, Config};

let client = LegendPrime::new(Config {
    query_key: std::env::var("LEGEND_QUERY_KEY").unwrap(),
    base_url: None, // defaults to https://prime-api.legend.xyz
    verbose: false,
});

// List accounts
let accounts = client.accounts.list().await?;

// View portfolio
let folio = client.accounts.folio("acc_xxx", &Default::default()).await?;

// Create and execute a plan
use legend_client::EarnParams;

let plan = client.plan.earn("acc_xxx", &EarnParams {
    amount: "1000".into(),
    asset: "usdc".into(),
    network: "base".into(),
    protocol: "aave_v3".into(),
    market: None,
}).await?;

let digest = plan.digest().expect("missing digest");
```

All plan types are supported: `earn`, `withdraw`, `swap`, `transfer`, `borrow`, `repay`, `claim_rewards`, `loop_long`, `unloop_long`, `add_backing`, `withdraw_backing`, `migrate`, `swap_and_supply`, `reinvest_rewards`.

## Error handling

```rust
use legend_client::LegendPrimeError;

match client.accounts.get("acc_xxx").await {
    Ok(account) => println!("{}", account.account_id),
    Err(LegendPrimeError::Api { code, message, status }) => {
        eprintln!("API error ({status}): [{code}] {message}");
    }
    Err(e) => eprintln!("{e}"),
}
```

## License

MIT
