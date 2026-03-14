use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub fn load_auth() -> Result<OpenCodeAuth, String> {
    let path = auth_path()?;

    let text = fs::read_to_string(&path).map_err(|e| format!("Could not read auth file: {}", e))?;

    let auth: OpenCodeAuth =
        serde_json::from_str(&text).map_err(|e| format!("Could not parse auth JSON: {}", e))?;

    if auth.opencode.is_none() {
        return Err("No OpenCode token found".to_string());
    }

    Ok(auth)
}

fn auth_path() -> Result<PathBuf, String> {
    if let Ok(opencode_home) = std::env::var("OPENCODE_HOME") {
        return Ok(PathBuf::from(opencode_home).join("auth.json"));
    }

    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("auth.json"))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OpenCodeAuth {
    #[serde(rename = "opencode")]
    pub opencode: Option<OpenCodeToken>,
    #[serde(rename = "google")]
    pub google: Option<GoogleToken>,
    #[serde(rename = "github-copilot")]
    pub github_copilot: Option<CopilotToken>,
    #[serde(rename = "openai")]
    pub openai: Option<OpenAIToken>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OpenCodeToken {
    #[serde(rename = "type")]
    pub token_type: Option<String>,
    pub key: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GoogleToken {
    #[serde(rename = "type")]
    pub token_type: Option<String>,
    pub refresh: Option<String>,
    pub access: Option<String>,
    pub expires: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CopilotToken {
    #[serde(rename = "type")]
    pub token_type: Option<String>,
    pub refresh: Option<String>,
    pub access: Option<String>,
    pub expires: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OpenAIToken {
    #[serde(rename = "type")]
    pub token_type: Option<String>,
    pub access: Option<String>,
}
