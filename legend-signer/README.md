# legend-signer

P256 signing library for [Legend](https://legend.xyz). Handles key generation, storage, and signing via multiple backends.

## Backends

| Backend | Platform | Storage |
|---|---|---|
| `KeychainSigner` | macOS | iCloud Keychain (syncs across Apple devices) |
| `FileSigner` | All | PEM file on disk |
| `TurnkeyClient` | All | Remote signing via [Turnkey](https://turnkey.com) API |

## Install

```toml
[dependencies]
legend-signer = "0.0.1"
```

## Usage

### File-based keys

```rust
use legend_signer::{FileSigner, Signer};
use std::path::Path;

// Generate a new key
let signer = FileSigner::generate(Path::new("key.pem"))?;
println!("Public key: {}", signer.public_key_hex());

// Load an existing key
let signer = FileSigner::load(Path::new("key.pem"))?;
let signature = signer.sign(b"message")?;
```

### macOS Keychain

```rust
#[cfg(target_os = "macos")]
{
    use legend_signer::KeychainSigner;

    let signer = KeychainSigner::generate("com.legend.cli.prod.default")?;
    println!("Public key: {}", signer.public_key_hex());

    // Keys sync via iCloud across all Apple devices
    let signer = KeychainSigner::load("com.legend.cli.prod.default")?;
    let signature = signer.sign(b"message")?;
}
```

### Turnkey (remote signing)

```rust
use legend_signer::{TurnkeyClient, TurnkeyConfig, FileSigner};

let signer = FileSigner::load(Path::new("key.pem"))?;
let turnkey = TurnkeyClient::new(TurnkeyConfig {
    signer: Box::new(signer),
    sub_org_id: "sub_org_xxx".into(),
    ethereum_signer_address: "0x742d...".into(),
    api_base_url: None,
    verbose: false,
});

let signature = turnkey.sign_digest("0xabc123...").await?;
```

## License

MIT
