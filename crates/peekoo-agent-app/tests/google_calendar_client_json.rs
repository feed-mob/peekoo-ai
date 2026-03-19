use peekoo_agent_app::parse_google_client_json;

#[test]
fn parses_installed_client_json() {
    let parsed = parse_google_client_json(
        r#"{
            "installed": {
                "client_id": "installed-client-id.apps.googleusercontent.com",
                "client_secret": "installed-secret"
            }
        }"#,
    )
    .expect("parse installed client json");

    assert_eq!(
        parsed.client_id,
        "installed-client-id.apps.googleusercontent.com"
    );
    assert_eq!(parsed.client_secret, "installed-secret");
}

#[test]
fn parses_web_client_json() {
    let parsed = parse_google_client_json(
        r#"{
            "web": {
                "client_id": "web-client-id.apps.googleusercontent.com",
                "client_secret": "web-secret"
            }
        }"#,
    )
    .expect("parse web client json");

    assert_eq!(parsed.client_id, "web-client-id.apps.googleusercontent.com");
    assert_eq!(parsed.client_secret, "web-secret");
}

#[test]
fn rejects_client_json_without_supported_shape() {
    let error =
        parse_google_client_json(r#"{"something_else":{}}"#).expect_err("shape should fail");

    assert!(error.contains("installed"));
}
