use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const REFRESH_URL: &str = "https://platform.claude.com/v1/oauth/token";
const SCOPES: &str = "user:profile user:inference user:sessions:claude_code user:mcp_servers";
const REFRESH_BUFFER_MS: u64 = 5 * 60 * 1000;

#[derive(Deserialize, Serialize)]
pub struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: OAuthData,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct OAuthData {
    #[serde(rename = "accessToken")]
    pub access_token: String,

    #[serde(rename = "refreshToken")]
    pub refresh_token: Option<String>,

    #[serde(rename = "expiresAt")]
    pub expires_at: Option<u64>,

    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<String>,
}

pub fn load_credentials() -> Result<CredentialsFile, String> {
    let path = credentials_path()?;
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(creds) = serde_json::from_str::<CredentialsFile>(&text) {
            return Ok(creds);
        }
    }

    let keychain_data = read_from_keychain()?;
    let creds: CredentialsFile = serde_json::from_str(&keychain_data)
        .map_err(|e| format!("Could not parse credentials JSON: {}", e))?;

    Ok(creds)
}

fn read_from_keychain() -> Result<String, String> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .map_err(|e| format!("Failed to read keychain: {}", e))?;

    if !output.status.success() {
        return Err("Keychain entry not found".to_string());
    }

    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if text.starts_with("7b") || text.contains("\\x") {
        text = decode_hex_string(&text)?;
    }

    Ok(text)
}

fn decode_hex_string(hex: &str) -> Result<String, String> {
    let hex = hex.trim();
    let mut bytes = Vec::new();
    let mut i = 0;

    while i < hex.len() {
        if hex.starts_with("\\x") {
            let byte = u8::from_str_radix(&hex[i + 2..i + 4], 16)
                .map_err(|_| "Invalid hex".to_string())?;
            bytes.push(byte);
            i += 4;
        } else if hex.len() >= 2 {
            let byte =
                u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| "Invalid hex".to_string())?;
            bytes.push(byte);
            i += 2;
        } else {
            break;
        }
    }

    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
}

pub fn is_token_expired(oauth: &OAuthData) -> bool {
    match oauth.expires_at {
        None => false,
        Some(expires_at_ms) => {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            now_ms + REFRESH_BUFFER_MS >= expires_at_ms
        }
    }
}

pub fn refresh_token(oauth: &mut OAuthData) -> Result<(), String> {
    let refresh_token = oauth
        .refresh_token
        .as_ref()
        .ok_or("No refresh token available")?;

    let client = reqwest::blocking::Client::new();

    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "client_id": CLIENT_ID,
        "scope": SCOPES,
    });

    let resp = client
        .post(REFRESH_URL)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Refresh request failed: {}", e))?;

    let status = resp.status();

    if status == 400 || status == 401 {
        return Err("Session expired. Run `claude` to log in again.".to_string());
    }
    if !status.is_success() {
        return Err(format!("Refresh failed: HTTP {}", status));
    }

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

    let new_token = data["access_token"]
        .as_str()
        .ok_or("Refresh response missing access_token")?;

    oauth.access_token = new_token.to_string();

    if let Some(new_refresh) = data["refresh_token"].as_str() {
        let new_refresh: &str = new_refresh;
        oauth.refresh_token = Some(new_refresh.to_string());
    }
    if let Some(expires_in) = data["expires_in"].as_u64() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        oauth.expires_at = Some(now_ms + expires_in * 1000);
    }

    Ok(())
}

fn credentials_path() -> Result<PathBuf, String> {
    if let Ok(custom) = std::env::var("CLAUDE_CREDS_PATH") {
        return Ok(PathBuf::from(custom));
    }

    let home = dirs::home_dir().ok_or("Could not find home directory")?;

    Ok(home.join(".claude").join(".credentials.json"))
}
