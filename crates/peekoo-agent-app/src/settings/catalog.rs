pub const OPENAI_COMPAT_PROVIDER_ID: &str = "openai-compatible";
pub const ANTHROPIC_COMPAT_PROVIDER_ID: &str = "anthropic-compatible";

pub fn default_api_for_provider(provider_id: &str) -> &'static str {
    match provider_id {
        OPENAI_COMPAT_PROVIDER_ID => "openai-completions",
        ANTHROPIC_COMPAT_PROVIDER_ID => "anthropic-messages",
        _ => "openai-completions",
    }
}

pub fn default_auth_header_for_provider(provider_id: &str) -> bool {
    match provider_id {
        OPENAI_COMPAT_PROVIDER_ID => true,
        ANTHROPIC_COMPAT_PROVIDER_ID => false,
        _ => true,
    }
}
