use crate::auth::codex_auth::CodexAuth;
use serde::Deserialize;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

#[derive(Deserialize, Debug)]
pub struct CodexUsageResponse {
    #[serde(rename = "rate_limit")]
    pub rate_limit: Option<CodexRateLimit>,
    #[serde(rename = "additional_rate_limits")]
    pub additional_rate_limits: Option<Vec<AdditionalRateLimit>>,
    #[serde(rename = "credits")]
    pub credits: Option<CodexCredits>,
    #[serde(rename = "plan_type")]
    pub plan_type: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CodexRateLimit {
    #[serde(rename = "primary_window")]
    pub primary_window: Option<RateWindow>,
    #[serde(rename = "secondary_window")]
    pub secondary_window: Option<RateWindow>,
}

#[derive(Deserialize, Debug)]
pub struct RateWindow {
    #[serde(rename = "used_percent")]
    pub used_percent: Option<f64>,
    #[serde(rename = "reset_at")]
    pub reset_at: Option<i64>,
    #[serde(rename = "reset_after_seconds")]
    pub reset_after_seconds: Option<i64>,
    #[serde(rename = "limit_window_seconds")]
    pub limit_window_seconds: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct AdditionalRateLimit {
    #[serde(rename = "limit_name")]
    pub limit_name: Option<String>,
    #[serde(rename = "rate_limit")]
    pub rate_limit: Option<CodexRateLimit>,
}

#[derive(Deserialize, Debug)]
pub struct CodexCredits {
    #[serde(rename = "balance")]
    pub balance: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct HeaderUsage {
    pub session: Option<f64>,
    pub weekly: Option<f64>,
    pub credits: Option<f64>,
}

pub fn fetch_usage(
    auth: &CodexAuth,
) -> Result<(CodexUsageResponse, HeaderUsage), Box<dyn std::error::Error>> {
    let tokens = auth.tokens.as_ref().ok_or("No tokens in codex auth")?;
    let access_token = &tokens.access_token;
    let account_id = tokens.account_id.as_deref();

    let client = reqwest::blocking::Client::new();
    let mut req = client
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .header("User-Agent", "OpenUsage");

    if let Some(account) = account_id {
        req = req.header("ChatGPT-Account-Id", account);
    }

    let resp = req.send()?;

    let status = resp.status();

    if status == 401 {
        return Err("Token expired. Run `codex` to log in again.".into());
    }
    if !status.is_success() {
        return Err(format!("API error: HTTP {}", status).into());
    }

    let header_session = resp
        .headers()
        .get("x-codex-primary-used-percent")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let header_weekly = resp
        .headers()
        .get("x-codex-secondary-used-percent")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let header_credits = resp
        .headers()
        .get("x-codex-credits-balance")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let header_usage = HeaderUsage {
        session: header_session,
        weekly: header_weekly,
        credits: header_credits,
    };

    let data: CodexUsageResponse = resp.json()?;

    Ok((data, header_usage))
}
