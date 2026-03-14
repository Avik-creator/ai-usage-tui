use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const REFRESH_AGE_MS: u64 = 8 * 24 * 60 * 60 * 1000;

#[derive(Deserialize, Serialize)]
pub struct CodexAuth {
    #[serde(rename = "tokens")]
    pub tokens: Option<CodexTokens>,
    #[serde(rename = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,
    #[serde(rename = "last_refresh")]
    pub last_refresh: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CodexTokens {
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "refresh_token")]
    pub refresh_token: Option<String>,
    #[serde(rename = "id_token")]
    pub id_token: Option<String>,
    #[serde(rename = "account_id")]
    pub account_id: Option<String>,
}

pub fn load_auth() -> Result<CodexAuth, String> {
    let path = auth_path()?;
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(auth) = serde_json::from_str::<CodexAuth>(&text) {
            if has_token_auth(&auth) {
                return Ok(auth);
            }
        }
    }

    let keychain_data = read_from_keychain()?;
    let auth: CodexAuth = serde_json::from_str(&keychain_data)
        .map_err(|e| format!("Could not parse codex auth JSON: {}", e))?;

    Ok(auth)
}

fn has_token_auth(auth: &CodexAuth) -> bool {
    auth.tokens
        .as_ref()
        .map(|t| !t.access_token.is_empty())
        .unwrap_or(false)
        || auth
            .openai_api_key
            .as_ref()
            .map(|k| !k.is_empty())
            .unwrap_or(false)
}

fn read_from_keychain() -> Result<String, String> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "Codex Auth", "-w"])
        .output()
        .map_err(|e| format!("Failed to read keychain: {}", e))?;

    if !output.status.success() {
        return Err("Codex keychain entry not found".to_string());
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

fn auth_path() -> Result<PathBuf, String> {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        return Ok(PathBuf::from(codex_home).join("auth.json"));
    }

    let home = dirs::home_dir().ok_or("Could not find home directory")?;

    let config_path = home.join(".config").join("codex").join("auth.json");
    if config_path.exists() {
        return Ok(config_path);
    }

    Ok(home.join(".codex").join("auth.json"))
}

pub fn needs_refresh(auth: &CodexAuth) -> bool {
    match &auth.last_refresh {
        None => true,
        Some(last) => {
            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(last) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let last_ms = last_time.timestamp_millis();
                (now - last_ms) > (REFRESH_AGE_MS as i64)
            } else {
                true
            }
        }
    }
}

pub fn refresh_token(auth: &mut CodexAuth) -> Result<(), String> {
    let refresh_token = auth
        .tokens
        .as_ref()
        .and_then(|t| t.refresh_token.as_ref())
        .ok_or("No refresh token available")?;

    let client = reqwest::blocking::Client::new();

    let body = format!(
        "grant_type=refresh_token&client_id={}&refresh_token={}",
        CLIENT_ID, refresh_token
    );

    let resp = client
        .post(REFRESH_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .map_err(|e| format!("Refresh request failed: {}", e))?;

    let status = resp.status();

    if status == 400 || status == 401 {
        return Err("Session expired. Run `codex` to log in again.".to_string());
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

    if let Some(tokens) = &mut auth.tokens {
        tokens.access_token = new_token.to_string();
        if let Some(new_refresh) = data["refresh_token"].as_str() {
            tokens.refresh_token = Some(new_refresh.to_string());
        }
        if let Some(id_token) = data["id_token"].as_str() {
            tokens.id_token = Some(id_token.to_string());
        }
    }

    auth.last_refresh = Some(chrono::Utc::now().to_rfc3339());

    Ok(())
}
