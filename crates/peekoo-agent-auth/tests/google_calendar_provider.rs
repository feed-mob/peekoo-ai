use peekoo_agent_auth::provider::google_calendar::build_authorize_url;
use peekoo_agent_auth::provider::google_calendar::{
    build_authorization_code_form_body, build_refresh_token_form_body,
};

#[test]
fn build_authorize_url_includes_google_calendar_scopes() {
    let url = build_authorize_url("client-id-123", "challenge123", "state123");

    assert!(url.contains("client_id=client-id-123"));
    assert!(url.contains("redirect_uri="));
    assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.readonly"));
    assert!(url.contains("access_type=offline"));
    assert!(url.contains("prompt=consent"));
    assert!(url.contains("state=state123"));
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"));
}

#[test]
fn authorization_code_form_body_includes_optional_client_secret() {
    let body = build_authorization_code_form_body(
        "client-id-123",
        Some("secret-456"),
        "auth-code",
        "verifier-789",
    );

    assert!(body.contains("client_id=client-id-123"));
    assert!(body.contains("client_secret=secret-456"));
    assert!(body.contains("code=auth-code"));
}

#[test]
fn refresh_token_form_body_omits_client_secret_when_missing() {
    let body = build_refresh_token_form_body("client-id-123", None, "refresh-token");

    assert!(body.contains("client_id=client-id-123"));
    assert!(body.contains("refresh_token=refresh-token"));
    assert!(!body.contains("client_secret="));
}
