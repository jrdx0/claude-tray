use serde::{Deserialize, Serialize};
use std::{fs, process::Command};

// This is the structure for the OAuth credentials of Claude AI
// stored in a file. It is used to authenticate requests to the Claude API.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAiOauth {
    // This is the access token for the OAuth credentials of Claude AI.
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub scopes: Vec<String>,
    pub subscription_type: String,
    pub rate_limit_tier: String,
}

// Wrapper for the OAuth credentials of Claude AI.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCredentials {
    pub claude_ai_oauth: ClaudeAiOauth,
}

// It represents the usage period of an account in detail.
// This struct is used inside the response of the Claude API
// usage endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct UsagePeriod {
    pub utilization: f32,
    pub resets_at: String,
}

// It is part of the response of the Claude API usage endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct ExtraUsage {
    pub is_enabled: bool,
    pub monthly_limit: Option<u64>,
    pub used_credits: Option<u64>,
    pub utilization: Option<f32>,
}

// It is the full response of the Claude API usage endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct ClaudeUsageResponse {
    // Information about the usage of the account (Current session on the tray).
    pub five_hour: UsagePeriod,
    // Information about the usage of the account (All models).
    pub seven_day: UsagePeriod,
    pub seven_day_oauth_apps: Option<UsagePeriod>,
    pub seven_day_opus: Option<UsagePeriod>,
    pub seven_day_sonnet: Option<UsagePeriod>,
    pub seven_day_iguana_necktie: Option<UsagePeriod>,
    pub extra_usage: ExtraUsage,
}

// Function to login to Claude API. It opens a terminal executing `claude /login`.
// When the user exits claude code execution, the terminal is closed and the
// function tries to get the credentials.
pub fn login() -> Result<ClaudeCredentials, String> {
    println!("Start login process");

    let mut claude_login_term = Command::new("x-terminal-emulator")
        .arg("-e")
        .arg("claude /login")
        .spawn()
        .expect("Failed to start terminal");

    claude_login_term
        .wait()
        .expect("Failed to wait for terminal");

    println!("Terminal closed. Verifying creation of credentials");

    get_credentials()
}

// Function to get the usage of the account. It receives the access token and returns the usage response.
pub async fn get_usage(access_token: &String) -> Result<ClaudeUsageResponse, String> {
    println!("Getting usage");

    let response = reqwest::Client::new()
        .get("https://api.anthropic.com/api/oauth/usage")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", access_token),
        )
        .header("anthropic-beta", "oauth-2025-04-20")
        .header(reqwest::header::USER_AGENT, "claude-code/2.0.61")
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .expect("Failed to send request");

    let response = response
        .json::<ClaudeUsageResponse>()
        .await
        .expect("Error parsing response to json");

    Ok(response)
}

// Function to get the credentials of the account. By default, the
// credentials are stored in a json file within the $HOME/.claude directory.
pub fn get_credentials() -> Result<ClaudeCredentials, String> {
    let env_home = std::env::var("HOME").expect("HOME environment variable not set");

    println!("Checking for credentials");

    let credentials = match fs::read_to_string(format!("{}/.claude/.credentials.json", env_home)) {
        Ok(file) => file,
        Err(_) => return Err("Failed to read credentials file".into()),
    };
    let credentials: ClaudeCredentials =
        serde_json::from_str(&credentials).expect("Invalid JSON format for credentials");

    println!("Credentials found");

    Ok(credentials)
}
