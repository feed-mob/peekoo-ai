use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("provider not supported: {0}")]
    UnsupportedProvider(String),
    #[error("oauth flow not found")]
    FlowNotFound,
    #[error("oauth callback listener bind failed: {0}")]
    CallbackBind(String),
    #[error("oauth callback listener error: {0}")]
    CallbackRead(String),
    #[error("oauth callback timed out")]
    CallbackTimeout,
    #[error("oauth provider returned error: {0}")]
    ProviderError(String),
    #[error("oauth missing authorization code")]
    MissingAuthorizationCode,
    #[error("oauth state mismatch")]
    StateMismatch,
    #[error("token exchange failed: {0}")]
    TokenExchange(String),
    #[error("invalid token response: {0}")]
    InvalidTokenResponse(String),
    #[error("flow lock error: {0}")]
    FlowLock(String),
}
