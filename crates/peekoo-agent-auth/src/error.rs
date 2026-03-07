use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("provider not supported: {0}")]
    UnsupportedProvider(String),
    #[error("token exchange failed: {0}")]
    TokenExchange(String),
    #[error("invalid token response: {0}")]
    InvalidTokenResponse(String),
    #[error("flow lock error: {0}")]
    FlowLock(String),
}
