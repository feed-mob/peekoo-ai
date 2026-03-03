use peekoo_agent_auth::{OAuthFlowStatus, OAuthService};

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
