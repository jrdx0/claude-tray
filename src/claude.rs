use crate::api::{ClaudeErrorResponse, ClaudeUsageResponse};
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use std::{fs, process::Command};

const CLAUDE_USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

// This is the structure for the OAuth credentials of Claude AI
// stored in a file. It is used to authenticate requests to the Claude API.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAiOauth {
    // This is the access token for the OAuth credentials of Claude AI.
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scopes: Option<Vec<String>>,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
}

// Wrapper for the OAuth credentials of Claude AI.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCredentials {
    pub claude_ai_oauth: ClaudeAiOauth,
}

#[derive(Debug, Clone)]
pub struct Claude {
    pub access_token: Option<String>,
}

impl Claude {
    pub fn new() -> Self {
        let mut instance = Self { access_token: None };

        if let Err(err) = instance.get_credentials() {
            error!("Failed to get credentials: {}", err);
        }

        instance
    }

    // Function to login to Claude API. It opens a terminal executing `claude /login`.
    // When the user exits claude code execution, the terminal is closed and the
    // function tries to get the credentials.
    pub fn login(&mut self) -> Result<(), String> {
        trace!("Start login process using gnome-terminal");

        let mut child_term = Command::new("gnome-terminal")
            .arg("-e")
            .arg("claude /login")
            .spawn()
            .map_err(|e| format!("Failed to spawn terminal: {}", e))?;

        child_term
            .wait()
            .map_err(|e| format!("Failed to wait for terminal: {}", e))?;

        trace!("Terminal closed. Verifying creation of credentials");

        if let Err(err) = self.get_credentials() {
            error!("Failed to get credentials: {}", err);
        }

        Ok(())
    }

    // Function to get the credentials of the account. By default, the
    // credentials are stored in a json file within the $HOME/.claude directory.
    pub fn get_credentials(&mut self) -> Result<(), String> {
        trace!("Getting $HOME environment variable");

        let env_home = std::env::var("HOME")
            .map_err(|e| format!("HOME environment variable not set: {}", e))?;

        trace!(
            "Reading credentials file located in {}/.claude/.credentials.json",
            env_home
        );

        let credentials = fs::read_to_string(format!("{}/.claude/.credentials.json", env_home))
            .map_err(|e| format!("Failed to read credentials file: {}", e))?;

        let credentials: ClaudeCredentials = serde_json::from_str(&credentials)
            .map_err(|e| format!("Error getting credentials: {}", e))?;

        info!(
            "Credentials found in {}/.claude/.credentials.json",
            env_home
        );

        self.access_token = credentials.claude_ai_oauth.access_token;

        if self.access_token.is_none() {
            return Err("Access token not found".to_string());
        }

        Ok(())
    }

    // Function to get the usage of the account. It receives the access token and returns the usage response.
    pub async fn get_usage(&mut self) -> Result<ClaudeUsageResponse, String> {
        trace!("Getting usage user information from {}", CLAUDE_USAGE_URL);

        let token = if let Some(token) = self.access_token.as_ref() {
            token
        } else {
            return Err("Access token not found when getting usage".to_string());
        };

        let response = reqwest::Client::new()
            .get(CLAUDE_USAGE_URL)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header(reqwest::header::USER_AGENT, "claude-code/2.0.61")
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| format!("Error requesting usage: {}", e))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Error reading response text: {}", e))?;

        info!("Request response (status {}): {}", status, response_text);

        // Try to parse as success response first
        if let Ok(usage) = serde_json::from_str::<ClaudeUsageResponse>(&response_text) {
            return Ok(usage);
        }

        if let Ok(error_response) = serde_json::from_str::<ClaudeErrorResponse>(&response_text) {
            return Err(format!(
                "API error ({}): {} [request_id: {}]",
                error_response.error.error_type,
                error_response.error.message,
                error_response.request_id
            ));
        }

        Err(format!("Unexpected API response format: {}", response_text))
    }
}
