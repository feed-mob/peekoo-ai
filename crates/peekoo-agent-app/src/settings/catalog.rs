use crate::settings::dto::ProviderCatalogDto;

pub const DEFAULT_PROVIDER: &str = "anthropic";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
pub const OPENAI_COMPAT_PROVIDER_ID: &str = "openai-compatible";
pub const ANTHROPIC_COMPAT_PROVIDER_ID: &str = "anthropic-compatible";

fn models_for_provider(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "anthropic" => &["claude-sonnet-4-6", "claude-opus-4-5"],
        "openai" => &["gpt-4o", "gpt-4.1"],
        "openai-codex" => &["gpt-5.3-codex"],
        _ => &[DEFAULT_MODEL],
    }
}

pub fn default_model_for_provider(provider_id: &str) -> &'static str {
    if provider_id == OPENAI_COMPAT_PROVIDER_ID {
        return "gpt-4o-mini";
    }
    if provider_id == ANTHROPIC_COMPAT_PROVIDER_ID {
        return "claude-3-5-haiku-latest";
    }

    models_for_provider(provider_id)
        .first()
        .copied()
        .unwrap_or(DEFAULT_MODEL)
}

pub fn normalize_model_for_provider(provider_id: &str, model_id: &str) -> String {
    if models_for_provider(provider_id).contains(&model_id) {
        return model_id.to_string();
    }
    default_model_for_provider(provider_id).to_string()
}

pub fn provider_catalog() -> Vec<ProviderCatalogDto> {
    vec![
        ProviderCatalogDto {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            auth_modes: vec!["api_key".into()],
            models: models_for_provider("anthropic")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "openai".into(),
            name: "OpenAI".into(),
            auth_modes: vec!["api_key".into()],
            models: models_for_provider("openai")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "openai-codex".into(),
            name: "OpenAI Codex".into(),
            auth_modes: vec!["oauth".into()],
            models: models_for_provider("openai-codex")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: OPENAI_COMPAT_PROVIDER_ID.into(),
            name: "OpenAI-Compatible".into(),
            auth_modes: vec!["api_key".into()],
            models: Vec::new(),
        },
        ProviderCatalogDto {
            id: ANTHROPIC_COMPAT_PROVIDER_ID.into(),
            name: "Anthropic-Compatible".into(),
            auth_modes: vec!["api_key".into()],
            models: Vec::new(),
        },
    ]
}

pub fn is_compatible_provider(provider_id: &str) -> bool {
    provider_id == OPENAI_COMPAT_PROVIDER_ID || provider_id == ANTHROPIC_COMPAT_PROVIDER_ID
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
    fn normalize_model_keeps_valid_model() {
        let normalized = normalize_model_for_provider("openai-codex", "gpt-5.3-codex");
        assert_eq!(normalized, "gpt-5.3-codex");
    }

    #[test]
    fn normalize_model_falls_back_for_invalid_pair() {
        let normalized = normalize_model_for_provider("openai-codex", "claude-sonnet-4-6");
        assert_eq!(normalized, "gpt-5.3-codex");
    }
}
