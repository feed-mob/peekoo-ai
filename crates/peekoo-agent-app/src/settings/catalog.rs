use crate::settings::dto::ProviderCatalogDto;

pub const DEFAULT_PROVIDER: &str = "pi-acp";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
pub const OPENAI_COMPAT_PROVIDER_ID: &str = "openai-compatible";
pub const ANTHROPIC_COMPAT_PROVIDER_ID: &str = "anthropic-compatible";

fn models_for_provider(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "pi-acp" => &["claude-sonnet-4-6", "claude-opus-4-5"],
        "opencode" => &["gpt-4.1", "gpt-4o"],
        "claude-code" => &["claude-sonnet-4-6", "claude-opus-4-5"],
        "codex" => &["gpt-5.3-codex"],
        _ => &[],
    }
}

pub fn default_model_for_provider(provider_id: &str) -> &'static str {
    models_for_provider(provider_id)
        .first()
        .copied()
        .unwrap_or(DEFAULT_MODEL)
}

pub fn normalize_model_for_provider(provider_id: &str, model_id: &str) -> String {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return default_model_for_provider(provider_id).to_string();
    }
    trimmed.to_string()
}

pub fn provider_catalog() -> Vec<ProviderCatalogDto> {
    vec![
        ProviderCatalogDto {
            id: "pi-acp".into(),
            name: "Peekoo ACP".into(),
            auth_modes: Vec::new(),
            models: models_for_provider("pi-acp")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "opencode".into(),
            name: "OpenCode".into(),
            auth_modes: Vec::new(),
            models: models_for_provider("opencode")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            auth_modes: Vec::new(),
            models: models_for_provider("claude-code")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "codex".into(),
            name: "Codex".into(),
            auth_modes: Vec::new(),
            models: models_for_provider("codex")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
    ]
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
        let normalized = normalize_model_for_provider("pi-acp", "");
        assert_eq!(normalized, default_model_for_provider("pi-acp"));
    }

    #[test]
    fn provider_catalog_lists_acp_providers() {
        let providers = provider_catalog();
        let ids: Vec<_> = providers.into_iter().map(|provider| provider.id).collect();

        assert_eq!(DEFAULT_PROVIDER, "pi-acp");
        assert!(ids.contains(&"pi-acp".to_string()));
        assert!(ids.contains(&"opencode".to_string()));
        assert!(ids.contains(&"claude-code".to_string()));
        assert!(ids.contains(&"codex".to_string()));
    }
}
