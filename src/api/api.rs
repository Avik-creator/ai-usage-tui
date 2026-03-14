use crate::auth::OauthData;
use serde::Deserialize;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Deserialize, Debug)]
pub struct UsageResponse {
    pub five_hour: Option<PeriodUsage>,
    pub seven_day: Option<PeriodUsage>,
    pub seven_day_sonnet: Option<PeriodUsage>
}

#[derive(Deserialize, Debug)]
pub struct PeriodUsage {
    pub utilization: f64,
    pub resets_at: Option<String>,
}

pub fn get_usage(oauth_data: &OauthData) -> Result<UsageResponse, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(USAGE_URL)
        .header(
            "Authorization",
            format!("Bearer {}", oauth.access_token.trim()),
        )
        .header("Accept", "application/json")
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "claude-code/2.1.69")
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

        let status = resp.status();

        if status == 401 {
        return Err("Token Expired, Run `claude` to re-authenticate".to_string() );
        }
        if !status.is_success() {
        return Err(format!("API error: HTTP {}", status));
    }

    let data: UsageResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)


}
