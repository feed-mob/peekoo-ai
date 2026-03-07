use crate::settings::dto::ProviderConfigDto;

pub fn ensure_pi_models_provider(cfg: &ProviderConfigDto, model_id: &str) -> Result<(), String> {
    let models_path = peekoo_paths::pi_models_path()?;
    let Some(parent) = models_path.parent() else {
        return Err(format!("Invalid pi models path: {}", models_path.display()));
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Create pi models dir error ({}): {e}", parent.display()))?;
    migrate_legacy_models_if_needed(&models_path)?;

    let mut root: serde_json::Value = if models_path.is_file() {
        let content = std::fs::read_to_string(&models_path)
            .map_err(|e| format!("Read pi models error ({}): {e}", models_path.display()))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Parse pi models error ({}): {e}", models_path.display()))?
    } else {
        serde_json::json!({ "providers": {} })
    };

    if !root.is_object() {
        root = serde_json::json!({ "providers": {} });
    }

    let providers = root
        .as_object_mut()
        .expect("root object")
        .entry("providers")
        .or_insert_with(|| serde_json::json!({}));
    if !providers.is_object() {
        *providers = serde_json::json!({});
    }

    providers.as_object_mut().expect("providers object").insert(
        cfg.provider_id.clone(),
        serde_json::json!({
            "baseUrl": cfg.base_url,
            "api": cfg.api,
            "authHeader": cfg.auth_header,
            "models": [
                {
                    "id": model_id,
                    "name": model_id
                }
            ]
        }),
    );

    let serialized = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Serialize pi models error ({}): {e}", models_path.display()))?;
    std::fs::write(&models_path, serialized)
        .map_err(|e| format!("Write pi models error ({}): {e}", models_path.display()))?;
    Ok(())
}

fn migrate_legacy_models_if_needed(models_path: &std::path::Path) -> Result<(), String> {
    if models_path.exists() {
        return Ok(());
    }

    let legacy_candidates = peekoo_paths::pi_legacy_models_paths();
    let Some(source) = legacy_candidates.iter().find(|path| path.is_file()) else {
        return Ok(());
    };

    std::fs::copy(source, models_path).map_err(|e| {
        format!(
            "Migrate pi models error ({} -> {}): {e}",
            source.display(),
            models_path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn ensure_provider_writes_to_pi_agent_dir_override() {
        let _guard = env_lock().lock().expect("env lock");

        let original = std::env::var_os("PI_CODING_AGENT_DIR");
        let test_dir = std::env::temp_dir().join(format!(
            "peekoo-pi-models-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));

        // SAFETY: test is serialized via mutex and restores env before returning.
        unsafe { std::env::set_var("PI_CODING_AGENT_DIR", &test_dir) };

        let cfg = ProviderConfigDto {
            provider_id: "openai-compatible".to_string(),
            base_url: "https://example.com/v1".to_string(),
            api: "openai-completions".to_string(),
            auth_header: true,
        };

        ensure_pi_models_provider(&cfg, "gpt-test").expect("write models");

        let models_path = test_dir.join("models.json");
        assert!(models_path.is_file());

        let content = std::fs::read_to_string(models_path).expect("read models");
        assert!(content.contains("openai-compatible"));
        assert!(content.contains("gpt-test"));

        let _ = std::fs::remove_dir_all(&test_dir);
        match original {
            Some(value) => {
                // SAFETY: restoring env state after test.
                unsafe { std::env::set_var("PI_CODING_AGENT_DIR", value) };
            }
            None => {
                // SAFETY: restoring env state after test.
                unsafe { std::env::remove_var("PI_CODING_AGENT_DIR") };
            }
        }
    }
}
