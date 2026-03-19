use peekoo_agent_app::parse_google_account_profile;

#[test]
fn parses_google_account_profile_response() {
    let profile = parse_google_account_profile(
        r#"{
            "email": "richard@example.com",
            "name": "Richard Roe",
            "picture": "https://example.com/avatar.png"
        }"#,
    )
    .expect("parse account profile");

    assert_eq!(profile.email, "richard@example.com");
    assert_eq!(profile.name.as_deref(), Some("Richard Roe"));
    assert_eq!(
        profile.picture.as_deref(),
        Some("https://example.com/avatar.png")
    );
}

#[test]
fn rejects_profile_without_email() {
    let error = parse_google_account_profile(r#"{"name":"No Email"}"#)
        .expect_err("missing email should fail");

    assert!(error.contains("email"));
}
