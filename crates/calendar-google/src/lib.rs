use url::Url;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
}

pub fn build_authorize_url(config: &OAuthConfig, state: &str, code_challenge: &str) -> Url {
    let mut url = Url::parse(GOOGLE_AUTH_URL).expect("valid google oauth url");
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &config.scope)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_includes_pkce_and_state() {
        let config = OAuthConfig {
            client_id: "client-1".to_string(),
            redirect_uri: "http://127.0.0.1:8787/oauth/callback".to_string(),
            scope: "openid email https://www.googleapis.com/auth/calendar".to_string(),
        };

        let url = build_authorize_url(&config, "state-123", "challenge-xyz");
        let query = url.query().expect("query exists");

        assert!(query.contains("code_challenge=challenge-xyz"));
        assert!(query.contains("code_challenge_method=S256"));
        assert!(query.contains("state=state-123"));
        assert!(query.contains("response_type=code"));
    }
}
