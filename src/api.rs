use base64::{Engine as _, engine::general_purpose};
use log::trace;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;

use crate::utils::extract_param_from_url;

pub const ANTHROPIC_AUTH_URL: &str = "https://claude.ai/oauth/authorize";

pub const ANTHROPIC_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";

pub const ANTHROPIC_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

pub const ANTHROPIC_AUTH_SCOPE: &str = "org:create_api_key user:profile user:inference";

pub const OAUTH_REDIRECT_PORT: u16 = 54545;

// Error details structure for Claude API error responses
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub error_visibility: String,
}

// Error structure for Claude API error responses
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    pub details: ErrorDetails,
}

// Top-level error response from Claude API
#[derive(Debug, Deserialize, Serialize)]
pub struct ClaudeErrorResponse {
    #[serde(rename = "type")]
    pub response_type: String, // "error"
    pub error: ApiError,
    pub request_id: String,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct AnthropicTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: String,
}

// Generates a code verifier for OAuth2 authorization.
pub fn generate_code_verifier() -> String {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    general_purpose::URL_SAFE_NO_PAD.encode(random_bytes)
}

// Generates a state for OAuth2 authorization.
pub fn generate_state() -> String {
    let random_bytes: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
    general_purpose::URL_SAFE_NO_PAD.encode(random_bytes)
}

// Generates a code challenge for OAuth2 authorization.
pub fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();

    general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

// Runs a localhost server to wait for the OAuth callback.
pub async fn wait_for_oauth_callback(expected_state: &str) -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", OAUTH_REDIRECT_PORT))
        .map_err(|e| format!("Failed to bind to port {}: {}", OAUTH_REDIRECT_PORT, e))?;

    trace!("OAuth callback listening on port {}", OAUTH_REDIRECT_PORT);

    // Waiting for a connection
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("Failed to accept connection: {}", e))?;

    // Reading a HTTP request
    let mut buffer = [0; 1024];
    stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read from stream: {}", e))?;

    let request = String::from_utf8_lossy(&buffer);

    let received_state = extract_param_from_url(&request, "state")?;

    if received_state != expected_state {
        return Err("State value is not the same".to_string());
    }

    let code = extract_param_from_url(&request, "code")?;

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Success</h1></body></html>";

    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("Failed to write to stream: {}", e))?;

    Ok(code)
}
