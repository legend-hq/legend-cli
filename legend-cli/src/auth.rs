use crate::config::{self, Env};

pub fn resolve_query_key(
    key_flag: &Option<String>,
    env: Env,
    profile_name: &str,
) -> Result<String, String> {
    // 1. CLI flag
    if let Some(key) = key_flag {
        return Ok(key.clone());
    }

    // 2. Environment variable
    if let Ok(key) = std::env::var("LEGEND_QUERY_KEY") {
        return Ok(key);
    }

    // 3. Profile file
    if let Some(profile) = config::load_profile(env, profile_name) {
        if let Some(key) = &profile.query_key {
            return Ok(key.clone());
        }
    }

    Err("No auth configured. Run `legend-cli login`, set LEGEND_QUERY_KEY, or use --key".into())
}

/// Resolve the base URL: --base-url flag overrides the env default.
pub fn resolve_base_url(base_url_flag: &Option<String>, env: Env) -> String {
    base_url_flag
        .clone()
        .unwrap_or_else(|| env.base_url().to_string())
}
