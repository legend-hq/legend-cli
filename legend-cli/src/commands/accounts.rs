use std::collections::HashMap;

use legend_client::*;
use legend_signer::*;

use crate::auth::resolve_query_key;
use crate::config::{self, *};
use crate::output::*;

pub async fn list(
    key: &Option<String>,
    env: Env,
    profile: &str,
    base_url: &Option<String>,
    verbose: bool,
    all: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let mut list = client.accounts.list().await?;
    let local_keys = super::keys::local_pubkeys(env);
    let accessibility: HashMap<String, bool> = list
        .accounts
        .iter()
        .map(|a| (a.account_id.clone(), check_accessible(a, &local_keys)))
        .collect();

    if !all {
        list.accounts
            .retain(|a| accessibility.get(&a.account_id).copied().unwrap_or(false));
    }

    print_account_list(&list, &accessibility, all, mode);
    Ok(())
}

pub async fn get(
    account_id: &str,
    key: &Option<String>,
    env: Env,
    profile: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key,
        base_url: base_url.clone(),
        verbose,
    });

    let account = client.accounts.get(account_id).await?;
    let local_keys = super::keys::local_pubkeys(env);
    let accessible = check_accessible(&account, &local_keys);
    print_account(&account, accessible, mode);
    Ok(())
}

pub async fn create(
    signer_type: &str,
    ethereum_signer: &Option<String>,
    solana_signer: &Option<String>,
    p256_public_key: &Option<String>,
    keygen: bool,
    name: &Option<String>,
    use_file_key: bool,
    key: &Option<String>,
    env: Env,
    profile_name: &str,
    base_url: &Option<String>,
    verbose: bool,
    mode: &OutputMode,
) -> anyhow::Result<()> {
    let query_key = resolve_query_key(key, env, profile_name).map_err(anyhow::Error::msg)?;
    let client = LegendPrime::new(Config {
        query_key: query_key.clone(),
        base_url: base_url.clone(),
        verbose,
    });

    if keygen {
        let effective_name = name.as_deref().unwrap_or(profile_name);

        let (signer, key_source, key_label, key_path) =
            generate_key(effective_name, use_file_key, env)?;

        let account = client
            .accounts
            .create(&CreateAccountParams {
                signer_type: "turnkey_p256".into(),
                p256_public_key: Some(signer.public_key_hex().to_string()),
                key_storage: Some(key_source.clone()),
                ..Default::default()
            })
            .await?;

        let profile = Profile {
            query_key: Some(query_key),
            key_source,
            key_label,
            key_path,
            p256_public_key: signer.public_key_hex().to_string(),
            sub_org_id: account.turnkey_sub_org_id.clone().unwrap_or_default(),
            ethereum_signer_address: account.ethereum_signer_address.clone().unwrap_or_default(),
            account_external_id: account.account_id.clone(),
        };
        save_profile(env, effective_name, &profile)?;

        if !matches!(mode, OutputMode::Quiet) {
            eprintln!("Profile '{effective_name}' saved ({env})");
        }

        let local_keys = super::keys::local_pubkeys(env);
        let accessible = check_accessible(&account, &local_keys);
        print_account(&account, accessible, mode);
    } else {
        let account = client
            .accounts
            .create(&CreateAccountParams {
                signer_type: signer_type.into(),
                ethereum_signer_address: ethereum_signer.clone(),
                solana_signer_address: solana_signer.clone(),
                p256_public_key: p256_public_key.clone(),
                ..Default::default()
            })
            .await?;

        let local_keys = super::keys::local_pubkeys(env);
        let accessible = check_accessible(&account, &local_keys);
        print_account(&account, accessible, mode);
    }

    Ok(())
}

fn generate_key(
    name: &str,
    use_file_key: bool,
    env: Env,
) -> anyhow::Result<(Box<dyn Signer>, String, Option<String>, Option<String>)> {
    if use_file_key {
        let dir = keys_dir(env);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}.key"));
        let signer = FileSigner::generate(&path)?;
        Ok((
            Box::new(signer),
            "file".to_string(),
            None,
            Some(path.to_string_lossy().to_string()),
        ))
    } else {
        #[cfg(feature = "keychain")]
        {
            let label = format!("com.legend.cli.{env}.{name}");
            let signer = KeychainSigner::generate(&label)?;
            Ok((Box::new(signer), "keychain".to_string(), Some(label), None))
        }
        #[cfg(not(feature = "keychain"))]
        {
            anyhow::bail!(
                "iCloud Keychain is not available in this build. Use --use-file-key,\n\
                 or install via `brew install legend-cli` for iCloud Keychain support."
            );
        }
    }
}

/// Returns true if the given account's signing key is accessible on this machine.
///
/// - passkey / ledger: always accessible (key lives in the cloud or hardware)
/// - everything else: checks whether the account's p256_public_key appears in the set of
///   locally available keys (probed from the Keychain and file store via `keys::local_pubkeys`)
pub fn check_accessible(account: &Account, local_keys: &std::collections::HashSet<String>) -> bool {
    match account.key_storage.as_deref() {
        Some("passkey") | Some("ledger") => return true,
        _ => {}
    }

    account
        .p256_public_key
        .as_deref()
        .map(|pk| local_keys.contains(&pk.to_ascii_lowercase()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use legend_client::types::Account;

    fn make_account(ethereum_signer_address: &str, key_storage: Option<&str>) -> Account {
        Account {
            account_id: "acc_1".to_string(),
            signer_type: None,
            ethereum_signer_address: Some(ethereum_signer_address.to_string()),
            p256_public_key: None,
            legend_wallet_address: None,
            solana_wallet_address: None,
            turnkey_sub_org_id: None,
            key_storage: key_storage.map(|s| s.to_string()),
            created_at: "2026-01-01".to_string(),
        }
    }

fn make_local_keys(pubkeys: &[&str]) -> std::collections::HashSet<String> {
        pubkeys.iter().map(|pk| pk.to_ascii_lowercase()).collect()
    }

    #[test]
    fn passkey_is_always_accessible() {
        let account = make_account("0xabc", Some("passkey"));
        assert!(check_accessible(&account, &make_local_keys(&[])));
    }

    #[test]
    fn ledger_is_always_accessible() {
        let account = make_account("0xabc", Some("ledger"));
        assert!(check_accessible(&account, &make_local_keys(&[])));
    }

    #[test]
    fn accessible_when_pubkey_in_local_keys() {
        let mut account = make_account("0xabc", Some("file"));
        account.p256_public_key = Some("0x02aabbcc".to_string());
        assert!(check_accessible(&account, &make_local_keys(&["0x02aabbcc"])));
    }

    #[test]
    fn accessible_when_pubkey_in_local_keys_case_insensitive() {
        let mut account = make_account("0xabc", Some("keychain"));
        account.p256_public_key = Some("0x02AABBCC".to_string());
        assert!(check_accessible(&account, &make_local_keys(&["0x02aabbcc"])));
    }

    #[test]
    fn inaccessible_when_pubkey_not_in_local_keys() {
        let mut account = make_account("0xabc", Some("file"));
        account.p256_public_key = Some("0x02aabbcc".to_string());
        assert!(!check_accessible(&account, &make_local_keys(&["0x02different"])));
    }

    #[test]
    fn inaccessible_when_no_p256_public_key() {
        let account = make_account("0xabc", Some("file"));
        assert!(!check_accessible(&account, &make_local_keys(&["0x02aabbcc"])));
    }

    #[test]
    fn inaccessible_when_local_keys_empty() {
        let mut account = make_account("0xabc", None);
        account.p256_public_key = Some("0x02aabbcc".to_string());
        assert!(!check_accessible(&account, &make_local_keys(&[])));
    }
}
