use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Env {
    Dev,
    Stage,
    Prod,
}

impl Env {
    pub fn base_url(&self) -> &'static str {
        match self {
            Env::Dev => "http://localhost:4477",
            Env::Stage => "https://prime-api.stage.legend.xyz",
            Env::Prod => "https://prime-api.legend.xyz",
        }
    }

    pub fn dir_name(&self) -> &'static str {
        match self {
            Env::Dev => "dev",
            Env::Stage => "stage",
            Env::Prod => "prod",
        }
    }
}

impl std::fmt::Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.dir_name())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub query_key: Option<String>,
    pub key_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    pub p256_public_key: String,
    pub sub_org_id: String,
    pub ethereum_signer_address: String,
    pub account_external_id: String,
}

fn legend_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".legend")
}

pub fn profiles_dir(env: Env) -> PathBuf {
    legend_dir().join(env.dir_name()).join("profiles")
}

pub fn keys_dir(env: Env) -> PathBuf {
    legend_dir().join(env.dir_name()).join("keys")
}

pub fn load_profile(env: Env, name: &str) -> Option<Profile> {
    let path = profiles_dir(env).join(format!("{name}.json"));
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_profile(env: Env, name: &str, profile: &Profile) -> std::io::Result<()> {
    let dir = profiles_dir(env);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(profile)?;
    std::fs::write(&path, &json)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Returns all profiles found in the profiles directory for the given environment.
/// Silently skips files that fail to parse.
pub fn list_profiles(env: Env) -> Vec<Profile> {
    let dir = profiles_dir(env);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? != "json" {
                return None;
            }
            let data = std::fs::read_to_string(&path).ok()?;
            serde_json::from_str(&data).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serializes tests that mutate HOME to prevent races under parallel test execution.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce()>(f: F) -> TempDir {
        let _guard = ENV_MUTEX.lock().unwrap();
        let dir = TempDir::new().unwrap();
        // SAFETY: single-threaded access enforced by ENV_MUTEX above.
        unsafe { std::env::set_var("HOME", dir.path()) };
        f();
        dir
    }

    #[test]
    fn list_profiles_returns_empty_for_missing_dir() {
        let _dir = with_temp_home(|| {
            let profiles = list_profiles(Env::Dev);
            assert!(profiles.is_empty());
        });
    }

    #[test]
    fn list_profiles_returns_saved_profile() {
        let _dir = with_temp_home(|| {
            let profile = Profile {
                query_key: None,
                key_source: "file".to_string(),
                key_label: None,
                key_path: Some("/tmp/test.key".to_string()),
                p256_public_key: "0x02ab".to_string(),
                sub_org_id: "org-1".to_string(),
                ethereum_signer_address: "0xabc".to_string(),
                account_external_id: "acc_1".to_string(),
            };
            save_profile(Env::Dev, "test", &profile).unwrap();
            let profiles = list_profiles(Env::Dev);
            assert_eq!(profiles.len(), 1);
            assert_eq!(profiles[0].ethereum_signer_address, "0xabc");
        });
    }
}
