use crate::auth::OAuthData;
use serde::Deserialize;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Deserialize, Debug)]
pub struct UsageResponse {
    pub five_hour: Option<PeriodUsage>,
    pub seven_day: Option<PeriodUsage>,
    pub seven_day_sonnet: Option<PeriodUsage>,
}

#[derive(Deserialize, Debug)]
pub struct PeriodUsage {
    pub utilization: f64,
    pub resets_at: Option<String>,
}

pub fn fetch_usage(oauth_data: &OAuthData) -> Result<UsageResponse, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(USAGE_URL)
        .header(
            "Authorization",
            format!("Bearer {}", oauth_data.access_token.trim()),
        )
        .header("Accept", "application/json")
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "claude-code/2.1.69")
        .send()?;

    let status = resp.status();

    if status == 401 {
        return Err("Token Expired, Run `claude` to re-authenticate".into());
    }
    if !status.is_success() {
        return Err(format!("API error: HTTP {}", status).into());
    }

    let data: UsageResponse = resp.json()?;

    Ok(data)
}

pub fn get_usage(oauth_data: &OAuthData) -> Result<UsageResponse, Box<dyn std::error::Error>> {
    fetch_usage(oauth_data)
}
