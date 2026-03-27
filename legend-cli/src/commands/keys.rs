use crate::config::{self, Env};
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
        anyhow::bail!("Keychain not available on this platform. Use `keygen --use-file-key`.");
    }
}

pub fn list(env: Env) -> anyhow::Result<()> {
    let mut count = 0;

    // Keychain keys (macOS only)
    #[cfg(target_os = "macos")]
    {
        let prefix = format!("com.legend.cli.{env}.");
        let keys = legend_signer::keychain::list_keys(&prefix)?;
        for (label, pubkey) in &keys {
            let name = label.strip_prefix(&prefix).unwrap_or(label);
            println!("{name}\tkeychain\t{pubkey}");
            count += 1;
        }
    }

    // File keys
    let keys_dir = config::keys_dir(env);
    if keys_dir.exists() {
        for entry in std::fs::read_dir(&keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("key") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?");
                match legend_signer::FileSigner::load(&path) {
                    Ok(signer) => {
                        println!("{name}\tfile\t{}", signer.public_key_hex());
                        count += 1;
                    }
                    Err(e) => eprintln!("warning: failed to load {}: {e}", path.display()),
                }
            }
        }
    }

    if count == 0 {
        eprintln!("No keys found for environment '{env}'.");
    } else {
        eprintln!("\n{count} key(s) found.");
    }
    Ok(())
}

pub fn sign(name: &str, digest: &str, env: Env) -> anyhow::Result<()> {
    let digest_bytes = hex::decode(digest.strip_prefix("0x").unwrap_or(digest))
        .map_err(|e| anyhow::anyhow!("Invalid hex digest: {e}"))?;

    // Try keychain first (macOS), then file
    #[cfg(target_os = "macos")]
    {
        let label = format!("com.legend.cli.{env}.{name}");
        if let Ok(signer) = legend_signer::KeychainSigner::load(&label) {
            let sig = signer.sign(&digest_bytes)?;
            println!("0x{}", hex::encode(&sig));
            return Ok(());
        }
    }

    let key_path = config::keys_dir(env).join(format!("{name}.key"));
    if key_path.exists() {
        let signer = legend_signer::FileSigner::load(&key_path)?;
        let sig = signer.sign(&digest_bytes)?;
        println!("0x{}", hex::encode(&sig));
        return Ok(());
    }

    anyhow::bail!("Key '{name}' not found in keychain or file store.");
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
