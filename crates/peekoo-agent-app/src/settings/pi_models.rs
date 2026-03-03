use crate::settings::dto::ProviderConfigDto;

pub fn ensure_pi_models_provider(cfg: &ProviderConfigDto, model_id: &str) -> Result<(), String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Cannot determine home directory".into());
    };
    let pi_dir = home.join(".pi");
    std::fs::create_dir_all(&pi_dir).map_err(|e| format!("Create ~/.pi dir error: {e}"))?;
    let models_path = pi_dir.join("models.json");

    let mut root: serde_json::Value = if models_path.is_file() {
        let content = std::fs::read_to_string(&models_path)
            .map_err(|e| format!("Read ~/.pi/models.json error: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Parse ~/.pi/models.json error: {e}"))?
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
        .map_err(|e| format!("Serialize ~/.pi/models.json error: {e}"))?;
    std::fs::write(&models_path, serialized)
        .map_err(|e| format!("Write ~/.pi/models.json error: {e}"))?;
    Ok(())
}
