use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn load_auth() -> Result<CopilotAuth, String> {
    // Try gh CLI first
    if let Ok(auth) = load_from_gh_cli() {
        return Ok(auth);
    }

    // Try file
    let path = auth_path()?;
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(auth) = serde_json::from_str::<CopilotAuth>(&text) {
            if auth
                .github_copilot
                .as_ref()
                .map(|t| !t.token.is_empty())
                .unwrap_or(false)
            {
                return Ok(auth);
            }
        }
    }

    Err("No GitHub Copilot token found. Run `gh auth login`.".to_string())
}

fn load_from_gh_cli() -> Result<CopilotAuth, String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|e| format!("Failed to run gh: {}", e))?;

    if !output.status.success() {
        return Err("gh auth not logged in".to_string());
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err("Empty token from gh".to_string());
    }

    Ok(CopilotAuth {
        github_copilot: Some(CopilotToken { token }),
    })
}

fn auth_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("auth.json"))
}

pub fn fetch_usage(auth: &CopilotAuth) -> Result<CopilotUsageResponse, Box<dyn std::error::Error>> {
    let token = auth
        .github_copilot
        .as_ref()
        .ok_or("No token found")?
        .token
        .as_str();

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get("https://api.github.com/copilot_internal/user")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/json")
        .header("Editor-Version", "vscode/1.96.2")
        .header("X-Github-Api-Version", "2025-04-01")
        .send()?;

    let status = resp.status();

    if status == 401 || status == 403 {
        return Err("Token expired. Run `gh auth login`.".into());
    }
    if !status.is_success() {
        return Err(format!("API error: HTTP {}", status).into());
    }

    let data: CopilotUsageResponse = resp.json()?;

    Ok(data)
}

#[derive(Deserialize, Serialize)]
pub struct CopilotAuth {
    #[serde(rename = "github-copilot")]
    pub github_copilot: Option<CopilotToken>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CopilotToken {
    #[serde(rename = "token")]
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct CopilotUsageResponse {
    #[serde(rename = "copilot_plan")]
    pub copilot_plan: Option<String>,
    #[serde(rename = "quota_reset_date")]
    pub quota_reset_date: Option<String>,
    #[serde(rename = "quota_snapshots")]
    pub quota_snapshots: Option<QuotaSnapshots>,
    #[serde(rename = "limited_user_quotas")]
    pub limited_user_quotas: Option<LimitedQuotas>,
    #[serde(rename = "monthly_quotas")]
    pub monthly_quotas: Option<MonthlyQuotas>,
    #[serde(rename = "limited_user_reset_date")]
    pub limited_user_reset_date: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct QuotaSnapshots {
    #[serde(rename = "chat")]
    pub chat: Option<Quota>,
    #[serde(rename = "completions")]
    pub completions: Option<Quota>,
    #[serde(rename = "premium_interactions")]
    pub premium_interactions: Option<Quota>,
}

#[derive(Deserialize, Debug)]
pub struct Quota {
    #[serde(rename = "percent_remaining")]
    pub percent_remaining: Option<f64>,
    #[serde(rename = "remaining")]
    pub remaining: Option<i64>,
    #[serde(rename = "limit")]
    pub limit: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct LimitedQuotas {
    #[serde(rename = "chat")]
    pub chat: Option<i64>,
    #[serde(rename = "completions")]
    pub completions: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct MonthlyQuotas {
    #[serde(rename = "chat")]
    pub chat: Option<i64>,
    #[serde(rename = "completions")]
    pub completions: Option<i64>,
}
