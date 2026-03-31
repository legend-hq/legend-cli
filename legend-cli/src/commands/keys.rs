use crate::config::{self, Env};
use legend_signer::Signer;
use std::collections::HashSet;

/// Returns the hex public keys of all keys accessible on this machine for the given environment.
/// Probes both the iCloud Keychain (if available) and the file-based key store.
pub fn local_pubkeys(env: Env) -> HashSet<String> {
    let mut pubkeys = HashSet::new();

    #[cfg(feature = "keychain")]
    if let Ok(keys) = list_keychain_keys(env) {
        for (_name, pubkey, _label) in keys {
            pubkeys.insert(pubkey.to_ascii_lowercase());
        }
    }

    if let Ok(keys) = list_file_keys(env) {
        for (_name, pubkey) in keys {
            pubkeys.insert(pubkey.to_ascii_lowercase());
        }
    }

    pubkeys
}

// --- File keys ---

fn create_file_key(name: &str, env: Env) -> anyhow::Result<()> {
    let dir = config::keys_dir(env);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.key"));
    if path.exists() {
        anyhow::bail!("Key '{name}' already exists at {}", path.display());
    }
    let signer = legend_signer::FileSigner::generate(&path)?;
    eprintln!("Key saved to {}", path.display());
    println!("{}", signer.public_key_hex());
    Ok(())
}

fn list_file_keys(env: Env) -> anyhow::Result<Vec<(String, String)>> {
    let keys_dir = config::keys_dir(env);
    let mut keys = Vec::new();
    if keys_dir.exists() {
        for entry in std::fs::read_dir(&keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("key") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                match legend_signer::FileSigner::load(&path) {
                    Ok(signer) => keys.push((name, signer.public_key_hex().to_string())),
                    Err(e) => eprintln!("warning: failed to load {}: {e}", path.display()),
                }
            }
        }
    }
    Ok(keys)
}

fn sign_file_key(name: &str, digest: &[u8], env: Env) -> anyhow::Result<Option<Vec<u8>>> {
    let key_path = config::keys_dir(env).join(format!("{name}.key"));
    if !key_path.exists() {
        return Ok(None);
    }
    let signer = legend_signer::FileSigner::load(&key_path)?;
    Ok(Some(signer.sign(digest)?))
}

fn delete_file_key(name: &str, env: Env) -> anyhow::Result<()> {
    let path = config::keys_dir(env).join(format!("{name}.key"));
    if !path.exists() {
        anyhow::bail!("File key '{name}' not found at {}", path.display());
    }
    std::fs::remove_file(&path)?;
    eprintln!("Deleted file key: {}", path.display());
    Ok(())
}

// --- Keychain keys ---

#[cfg(feature = "keychain")]
fn keychain_label(name: &str, env: Env) -> String {
    format!("com.legend.cli.{env}.{name}")
}

#[cfg(feature = "keychain")]
fn keychain_prefix(env: Env) -> String {
    format!("com.legend.cli.{env}.")
}

#[cfg(feature = "keychain")]
fn create_keychain_key(name: &str, env: Env) -> anyhow::Result<()> {
    let label = keychain_label(name, env);
    let signer = legend_signer::KeychainSigner::generate(&label)?;
    eprintln!("Key created in iCloud Keychain (label: {label})");
    println!("{}", signer.public_key_hex());
    Ok(())
}

/// Returns (name, pubkey, label) tuples.
#[cfg(feature = "keychain")]
fn list_keychain_keys(env: Env) -> anyhow::Result<Vec<(String, String, String)>> {
    let prefix = keychain_prefix(env);
    let raw = legend_signer::keychain::list_keys(&prefix)?;
    Ok(raw
        .into_iter()
        .map(|(label, pubkey)| {
            let name = label
                .strip_prefix(&prefix)
                .unwrap_or(&label)
                .to_string();
            (name, pubkey, label)
        })
        .collect())
}

#[cfg(feature = "keychain")]
fn sign_keychain_key(name: &str, digest: &[u8], env: Env) -> anyhow::Result<Option<Vec<u8>>> {
    let label = keychain_label(name, env);
    match legend_signer::KeychainSigner::load(&label) {
        Ok(signer) => Ok(Some(signer.sign(digest)?)),
        Err(_) => Ok(None),
    }
}

#[cfg(feature = "keychain")]
fn delete_keychain_key(name: &str, env: Env) -> anyhow::Result<()> {
    let label = keychain_label(name, env);
    legend_signer::keychain::delete_key(&label)?;
    eprintln!("Deleted keychain key: {label}");
    Ok(())
}

// --- Dispatch helper ---

fn dispatch_default(
    file: bool,
    keychain: bool,
    file_fn: impl FnOnce() -> anyhow::Result<()>,
    #[cfg(feature = "keychain")] keychain_fn: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if file {
        return file_fn();
    }
    if keychain {
        #[cfg(feature = "keychain")]
        return keychain_fn();

        #[cfg(not(feature = "keychain"))]
        anyhow::bail!(
            "iCloud Keychain is not available in this build.\n\
             Install via `brew install legend-cli` for iCloud Keychain support."
        );
    }

    #[cfg(feature = "keychain")]
    return keychain_fn();

    #[cfg(not(feature = "keychain"))]
    return file_fn();
}

// --- Public commands ---

pub fn create(name: &str, env: Env, file: bool, keychain: bool) -> anyhow::Result<()> {
    dispatch_default(
        file,
        keychain,
        || create_file_key(name, env),
        #[cfg(feature = "keychain")]
        || create_keychain_key(name, env),
    )
}

pub fn list(env: Env, verbose: bool) -> anyhow::Result<()> {
    let mut count = 0;

    #[cfg(feature = "keychain")]
    for (name, pubkey, label) in list_keychain_keys(env)? {
        println!("{name}\tkeychain\t{pubkey}");
        if verbose {
            if let Ok(attrs) = legend_signer::keychain::key_attributes(&label) {
                eprintln!("  attrs: {attrs}");
            }
        }
        count += 1;
    }

    #[cfg(not(feature = "keychain"))]
    let _ = verbose;

    for (name, pubkey) in list_file_keys(env)? {
        println!("{name}\tfile\t{pubkey}");
        count += 1;
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

    #[cfg(feature = "keychain")]
    if let Some(sig) = sign_keychain_key(name, &digest_bytes, env)? {
        println!("0x{}", hex::encode(&sig));
        return Ok(());
    }

    if let Some(sig) = sign_file_key(name, &digest_bytes, env)? {
        println!("0x{}", hex::encode(&sig));
        return Ok(());
    }

    anyhow::bail!("Key '{name}' not found.");
}

pub fn delete(name: &str, env: Env, file: bool, keychain: bool) -> anyhow::Result<()> {
    dispatch_default(
        file,
        keychain,
        || delete_file_key(name, env),
        #[cfg(feature = "keychain")]
        || delete_keychain_key(name, env),
    )
}
