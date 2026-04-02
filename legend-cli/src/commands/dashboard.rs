use crate::config::{self, Env};

pub fn open(env: Env, profile_name: &str) -> anyhow::Result<()> {
    let profile = config::load_profile(env, profile_name)
        .ok_or_else(|| anyhow::anyhow!("No profile found. Run: legend-cli login"))?;

    let token = profile
        .query_key
        .ok_or_else(|| anyhow::anyhow!("No token in profile. Run: legend-cli login"))?;

    if !token.starts_with("eyJ") {
        anyhow::bail!(
            "Profile uses a query key, not a JWT. Run `legend-cli login` to get a \
             dashboard-compatible token."
        );
    }

    let url = format!(
        "{}/auth/callback?token={}",
        env.dashboard_url(),
        token
    );

    eprintln!("Opening dashboard...");
    if open::that(&url).is_err() {
        eprintln!("Could not open browser. Visit:\n{}", env.dashboard_url());
    }

    Ok(())
}
