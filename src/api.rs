use serde::{Deserialize, Serialize};

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
