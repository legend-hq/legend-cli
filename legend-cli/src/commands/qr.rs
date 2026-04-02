use legend_client::types::Account;
use qrcode::QrCode;

/// Map a network name to the corresponding address on an Account.
pub fn address_for_network(account: &Account, network: &str) -> anyhow::Result<String> {
    let addr = match network {
        "ethereum" => &account.ethereum_signer_address,
        "solana" => &account.solana_wallet_address,
        "legend" => &account.legend_wallet_address,
        _ => anyhow::bail!(
            "Unknown network \"{network}\". Valid networks: ethereum, solana, legend"
        ),
    };
    addr.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Account {} has no address for network \"{network}\"",
            account.account_id
        )
    })
}

/// Generate a terminal-renderable QR code string for the given data.
pub fn generate_qr_string(data: &str) -> anyhow::Result<String> {
    let code = QrCode::new(data.as_bytes())?;
    let string = code
        .render::<char>()
        .quiet_zone(false)
        .module_dimensions(2, 1)
        .build();
    Ok(string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account(
        ethereum: Option<&str>,
        solana: Option<&str>,
        legend: Option<&str>,
    ) -> Account {
        Account {
            account_id: "acc_test".to_string(),
            signer_type: None,
            ethereum_signer_address: ethereum.map(|s| s.to_string()),
            p256_public_key: None,
            legend_wallet_address: legend.map(|s| s.to_string()),
            solana_wallet_address: solana.map(|s| s.to_string()),
            turnkey_sub_org_id: None,
            key_storage: None,
            created_at: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn address_for_ethereum() {
        let account = make_account(Some("0xABCD"), None, None);
        assert_eq!(address_for_network(&account, "ethereum").unwrap(), "0xABCD");
    }

    #[test]
    fn address_for_solana() {
        let account = make_account(None, Some("So1ana"), None);
        assert_eq!(address_for_network(&account, "solana").unwrap(), "So1ana");
    }

    #[test]
    fn address_for_legend() {
        let account = make_account(None, None, Some("legend_addr"));
        assert_eq!(
            address_for_network(&account, "legend").unwrap(),
            "legend_addr"
        );
    }

    #[test]
    fn unknown_network_errors() {
        let account = make_account(Some("0xABCD"), None, None);
        let err = address_for_network(&account, "bitcoin").unwrap_err();
        assert!(err.to_string().contains("Unknown network"));
    }

    #[test]
    fn missing_address_errors() {
        let account = make_account(None, None, None);
        let err = address_for_network(&account, "ethereum").unwrap_err();
        assert!(err.to_string().contains("no address"));
    }

    #[test]
    fn generate_qr_produces_output() {
        let qr = generate_qr_string("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18").unwrap();
        assert!(!qr.is_empty());
        assert!(qr.contains('\u{2588}')); // full block character
    }

    #[test]
    fn generate_qr_is_deterministic() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18";
        let a = generate_qr_string(addr).unwrap();
        let b = generate_qr_string(addr).unwrap();
        assert_eq!(a, b);
    }
}
