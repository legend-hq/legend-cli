use legend_client::*;
use legend_signer::*;

use crate::auth::resolve_query_key;
use crate::config::*;
use crate::output::*;

pub async fn list(
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

    let list = client.accounts.list().await?;
    print_account_list(&list, mode);
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
    print_account(&account, mode);
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

        print_account(&account, mode);
    } else {
        let account = client
            .accounts
            .create(&CreateAccountParams {
                signer_type: signer_type.into(),
                ethereum_signer_address: ethereum_signer.clone(),
                solana_signer_address: solana_signer.clone(),
                p256_public_key: p256_public_key.clone(),
            })
            .await?;

        print_account(&account, mode);
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
