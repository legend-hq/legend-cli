use crate::config::Env;
use legend_signer::Signer;

pub fn create(name: &str, env: Env) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let label = format!("com.legend.cli.{env}.{name}");
        let signer = legend_signer::KeychainSigner::generate(&label)?;
        eprintln!("Key created in iCloud Keychain (label: {label})");
        println!("{}", signer.public_key_hex());
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (name, env);
        anyhow::bail!("Keychain not available on this platform.");
    }
}

pub fn list(env: Env) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let prefix = format!("com.legend.cli.{env}.");
        let keys = legend_signer::keychain::list_keys(&prefix)?;

        if keys.is_empty() {
            eprintln!("No keys found for environment '{env}'.");
            return Ok(());
        }

        for (label, pubkey) in &keys {
            let name = label.strip_prefix(&prefix).unwrap_or(label);
            println!("{name}\t{pubkey}");
        }
        eprintln!("\n{} key(s) found.", keys.len());
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = env;
        anyhow::bail!("Keychain not available on this platform.");
    }
}

pub fn sign(name: &str, digest: &str, env: Env) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let label = format!("com.legend.cli.{env}.{name}");
        let signer = legend_signer::KeychainSigner::load(&label)?;

        let digest_bytes = hex::decode(digest.strip_prefix("0x").unwrap_or(digest))
            .map_err(|e| anyhow::anyhow!("Invalid hex digest: {e}"))?;

        let sig = signer.sign(&digest_bytes)?;
        println!("0x{}", hex::encode(&sig));
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (name, digest, env);
        anyhow::bail!("Keychain not available on this platform.");
    }
}

pub fn delete(name: &str, env: Env) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let label = format!("com.legend.cli.{env}.{name}");
        legend_signer::keychain::delete_key(&label)?;
        eprintln!("Deleted key: {label}");
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (name, env);
        anyhow::bail!("Keychain not available on this platform.");
    }
}
