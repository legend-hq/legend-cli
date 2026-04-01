use std::path::Path;

use legend_signer::*;

use crate::config::{self, Env, Profile};

fn check_profile_accessible(profile: &Profile, env: Env) -> anyhow::Result<()> {
    let local_keys = super::keys::local_pubkeys(env);
    if !local_keys.contains(&profile.p256_public_key.to_ascii_lowercase()) {
        anyhow::bail!(
            "Signing key for this profile is not accessible on this machine. \
            Run `legend-cli accounts list` to see accessible accounts."
        );
    }
    Ok(())
}

pub async fn sign(digest: &str, env: Env, profile_name: &str, verbose: bool) -> anyhow::Result<()> {
    let profile = config::load_profile(env, profile_name)
        .ok_or_else(|| anyhow::anyhow!("No profile found. Run: legend-cli login"))?;

    check_profile_accessible(&profile, env)?;

    let signer = load_signer_from_profile(&profile)?;
    let turnkey = TurnkeyClient::new(TurnkeyConfig {
        signer,
        sub_org_id: profile.sub_org_id,
        ethereum_signer_address: profile.ethereum_signer_address,
        api_base_url: None,
        verbose,
    });

    let signature = turnkey.sign_digest(digest).await?;
    println!("{signature}");
    Ok(())
}

pub fn load_signer_from_profile(profile: &Profile) -> anyhow::Result<Box<dyn Signer>> {
    match profile.key_source.as_str() {
        "keychain" => {
            #[cfg(feature = "keychain")]
            {
                let label = profile
                    .key_label
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Profile missing key_label for Keychain"))?;
                Ok(Box::new(KeychainSigner::load(label, Some(&profile.p256_public_key))?))
            }
            #[cfg(not(feature = "keychain"))]
            {
                anyhow::bail!(
                    "iCloud Keychain is not available in this build.\n\
                     Install via `brew install legend-cli` for iCloud Keychain support."
                )
            }
        }
        // Legacy: support old profiles that used secure_enclave
        "secure_enclave" => {
            anyhow::bail!(
                "Secure Enclave profiles are no longer supported. Re-run: legend-cli accounts create --keygen"
            )
        }
        "file" => {
            let path = profile
                .key_path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Profile missing key_path for file signer"))?;
            Ok(Box::new(FileSigner::load(Path::new(path))?))
        }
        "none" => {
            anyhow::bail!("No signing key configured. Run: legend-cli accounts create --keygen")
        }
        other => anyhow::bail!("Unknown key source: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;

    fn file_profile(key_path: &str) -> Profile {
        Profile {
            query_key: None,
            key_source: "file".to_string(),
            key_label: None,
            key_path: Some(key_path.to_string()),
            p256_public_key: "0x02ab".to_string(),
            sub_org_id: "org".to_string(),
            ethereum_signer_address: "0xabc".to_string(),
            account_external_id: "acc_1".to_string(),
        }
    }

    #[test]
    fn file_signer_load_errors_for_missing_file() {
        // This tests FileSigner::load directly via load_signer_from_profile.
        // The file-existence guard in sign() provides the user-facing error message;
        // this test confirms the underlying signer load also returns an error.
        let profile = file_profile("/nonexistent/path/key.key");
        let result = load_signer_from_profile(&profile);
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        // The error comes from FileSigner::load — should mention the path or file not found
        assert!(
            msg.contains("key file") || msg.contains("not found") || msg.contains("No such file") || msg.contains("nonexistent"),
            "unexpected error: {msg}"
        );
    }
}
