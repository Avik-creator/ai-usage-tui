use crate::auth::codex_auth::CodexAuth;
use serde::Deserialize;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

#[derive(Deserialize, Debug)]
pub struct CodexUsageResponse {
    #[serde(rename = "user_id")]
    pub user_id: Option<String>,
    #[serde(rename = "account_id")]
    pub account_id: Option<String>,
    #[serde(rename = "email")]
    pub email: Option<String>,
    #[serde(rename = "plan_type")]
    pub plan_type: Option<String>,
    #[serde(rename = "rate_limit")]
    pub rate_limit: Option<CodexRateLimit>,
}

#[derive(Deserialize, Debug)]
pub struct CodexRateLimit {
    #[serde(rename = "allowed")]
    pub allowed: Option<bool>,
    #[serde(rename = "limit_reached")]
    pub limit_reached: Option<bool>,
    #[serde(rename = "primary_window")]
    pub primary_window: Option<RateWindow>,
    #[serde(rename = "secondary_window")]
    pub secondary_window: Option<RateWindow>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum RateWindow {
    Value(RateWindowData),
    Null,
}

#[derive(Deserialize, Debug)]
pub struct RateWindowData {
    #[serde(rename = "used_percent")]
    pub used_percent: Option<f64>,
    #[serde(rename = "reset_at")]
    pub reset_at: Option<i64>,
    #[serde(rename = "reset_after_seconds")]
    pub reset_after_seconds: Option<i64>,
    #[serde(rename = "limit_window_seconds")]
    pub limit_window_seconds: Option<i64>,
}

impl RateWindow {
    pub fn as_data(&self) -> Option<&RateWindowData> {
        match self {
            RateWindow::Value(d) => Some(d),
            RateWindow::Null => None,
        }
    }
}

pub struct HeaderUsage {
    pub session: Option<f64>,
    pub weekly: Option<f64>,
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
        .header("User-Agent", "ClaudeCode/2.1.69");

    if let Some(account) = account_id {
        req = req.header("ChatGPT-Account-Id", account);
    }

    let resp = req.send()?;

    let status = resp.status();

    if status == 401 {
        return Err("Token expired. Run `codex` to log in again.".into());
    }

    if status == 403 {
        return Err("Access forbidden. Your Codex subscription may have expired.".into());
    }

    if !status.is_success() {
        return Err(format!("API error: HTTP {}", status).into());
    }

    let data: CodexUsageResponse = resp.json()?;

    let session = data
        .rate_limit
        .as_ref()
        .and_then(|rl| rl.primary_window.as_ref())
        .and_then(|pw| pw.as_data())
        .and_then(|d| d.used_percent);

    let weekly = data
        .rate_limit
        .as_ref()
        .and_then(|rl| rl.secondary_window.as_ref())
        .and_then(|sw| sw.as_data())
        .and_then(|d| d.used_percent);

    Ok((data, HeaderUsage { session, weekly }))
}
