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
