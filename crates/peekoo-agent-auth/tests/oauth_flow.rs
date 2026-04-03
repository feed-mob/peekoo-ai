use peekoo_agent_auth::{
    OAuthFlowStatus, OAuthQueryParam, OAuthService, OAuthStartConfig, OAuthTokenExchangeConfig,
};

#[tokio::test(flavor = "current_thread")]
async fn status_for_unknown_flow_is_expired() {
    let service = OAuthService::new();
    let status = service.status("does-not-exist").await.expect("status");
    assert_eq!(status.status, OAuthFlowStatus::Expired);
}

#[test]
fn unsupported_provider_returns_error() {
    let service = OAuthService::new();
    let result = service.start("unknown-provider");
    assert!(result.is_err());
}

#[test]
fn custom_start_builds_authorize_url_with_standard_and_extra_params() {
    let service = OAuthService::new();
    let started = service
        .start_custom(OAuthStartConfig {
            provider_id: "google-calendar".to_string(),
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            client_id: "client-id".to_string(),
            client_secret: Some("client-secret".to_string()),
            redirect_uri: "http://localhost:1455/auth/callback".to_string(),
            scope: "openid email profile".to_string(),
            authorize_params: vec![
                OAuthQueryParam::new("access_type", "offline"),
                OAuthQueryParam::new("prompt", "consent"),
            ],
            token_exchange: OAuthTokenExchangeConfig {
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                token_params: vec![],
            },
        })
        .expect("custom oauth start");

    assert!(
        started
            .authorize_url
            .starts_with("https://accounts.google.com/o/oauth2/v2/auth?")
    );
    assert!(started.authorize_url.contains("response_type=code"));
    assert!(started.authorize_url.contains("client_id=client-id"));
    // Redirect URI should be localhost with a port between 1455-1465
    assert!(
        started
            .authorize_url
            .contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A")
    );
    assert!(
        started
            .authorize_url
            .contains("scope=openid%20email%20profile")
    );
    assert!(started.authorize_url.contains("code_challenge_method=S256"));
    assert!(started.authorize_url.contains("access_type=offline"));
    assert!(started.authorize_url.contains("prompt=consent"));
}
