#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthFlowStatus {
    Pending,
    Completed,
    Failed,
    Expired,
}

impl OAuthFlowStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OAuthFlow {
    pub provider_id: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub verifier: String,
    pub auth_code: Option<String>,
    pub status: OAuthFlowStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthStartResult {
    pub flow_id: String,
    pub authorize_url: String,
}

#[derive(Debug, Clone)]
pub struct OAuthStatusResult {
    pub provider_id: String,
    pub status: OAuthFlowStatus,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}
