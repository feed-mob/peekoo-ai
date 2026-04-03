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
    pub start_config: OAuthStartConfig,
    pub verifier: String,
    pub auth_code: Option<String>,
    pub status: OAuthFlowStatus,
    pub error: Option<String>,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthQueryParam {
    pub key: String,
    pub value: String,
}

impl OAuthQueryParam {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthTokenExchangeConfig {
    pub token_url: String,
    pub token_params: Vec<OAuthQueryParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthStartConfig {
    pub provider_id: String,
    pub authorize_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scope: String,
    pub authorize_params: Vec<OAuthQueryParam>,
    pub token_exchange: OAuthTokenExchangeConfig,
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
