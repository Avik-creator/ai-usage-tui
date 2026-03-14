use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn load_auth() -> Result<OpenCodeAuth, String> {
    if let Ok(path) = auth_path() {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(auth) = serde_json::from_str::<OpenCodeAuth>(&text) {
                return Ok(auth);
            }
        }
    }

    let keychain_data = read_from_keychain()?;
    let auth: OpenCodeAuth = serde_json::from_str(&keychain_data)
        .map_err(|e| format!("Could not parse OpenCode auth JSON: {}", e))?;

    Ok(auth)
}

fn read_from_keychain() -> Result<String, String> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "OpenCode", "-w"])
        .output()
        .map_err(|e| format!("Failed to read keychain: {}", e))?;

    if !output.status.success() {
        return Err("OpenCode keychain entry not found".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(text)
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
    #[serde(flatten)]
    pub extra: serde_json::Value,
}
