pub const DEFAULT_PROVIDER: &str = "opencode";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
pub const OPENAI_COMPAT_PROVIDER_ID: &str = "openai-compatible";
pub const ANTHROPIC_COMPAT_PROVIDER_ID: &str = "anthropic-compatible";

pub fn default_model_for_provider(_provider_id: &str) -> &'static str {
    // Models are discovered via ACP protocol, not hardcoded.
    // Return the global default for backward compatibility.
    DEFAULT_MODEL
}

pub fn normalize_model_for_provider(provider_id: &str, model_id: &str) -> String {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return default_model_for_provider(provider_id).to_string();
    }
    trimmed.to_string()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_model_keeps_explicit_model_for_acp_provider() {
        let normalized = normalize_model_for_provider("codex", "gpt-5.3-codex");
        assert_eq!(normalized, "gpt-5.3-codex");
    }

    #[test]
    fn normalize_model_falls_back_when_empty() {
        let normalized = normalize_model_for_provider("opencode", "");
        assert_eq!(normalized, default_model_for_provider("opencode"));
    }
}
