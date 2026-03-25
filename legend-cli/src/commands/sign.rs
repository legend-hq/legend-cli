use std::path::Path;

use legend_signer::*;

use crate::config::{self, Env, Profile};

pub async fn sign(digest: &str, env: Env, profile_name: &str, verbose: bool) -> anyhow::Result<()> {
    let profile = config::load_profile(env, profile_name)
        .ok_or_else(|| anyhow::anyhow!("No profile found. Run: legend-cli login"))?;

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
            #[cfg(target_os = "macos")]
            {
                let label = profile
                    .key_label
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Profile missing key_label for Keychain"))?;
                Ok(Box::new(KeychainSigner::load(label)?))
            }
            #[cfg(not(target_os = "macos"))]
            {
                anyhow::bail!("macOS Keychain not available on this platform")
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
