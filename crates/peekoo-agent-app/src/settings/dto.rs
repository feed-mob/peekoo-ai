use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SkillInstallOutcome {
    Installed {
        skill: SkillDto,
    },
    Conflict {
        #[serde(rename = "skillId")]
        skill_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthDto {
    pub provider_id: String,
    pub auth_mode: String,
    pub configured: bool,
    pub oauth_expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub skill_id: String,
    pub source_type: String,
    pub path: String,
    pub enabled: bool,
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigDto {
    pub provider_id: String,
    pub base_url: String,
    pub api: String,
    pub auth_header: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettingsDto {
    pub system_prompt: Option<String>,
    pub max_tool_iterations: usize,
    pub version: i64,
    pub provider_auth: Vec<ProviderAuthDto>,
    pub provider_configs: Vec<ProviderConfigDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettingsPatchDto {
    pub system_prompt: Option<String>,
    pub max_tool_iterations: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogDto {
    pub id: String,
    pub name: String,
    pub auth_modes: Vec<String>,
    pub models: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettingsCatalogDto {
    pub providers: Vec<ProviderCatalogDto>,
    pub discovered_skills: Vec<SkillDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetApiKeyRequest {
    pub provider_id: String,
    pub api_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProviderConfigRequest {
    pub provider_id: String,
    pub base_url: String,
    pub api: Option<String>,
    pub auth_header: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRequest {
    pub provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthStatusRequest {
    pub flow_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthStartResponse {
    pub flow_id: String,
    pub authorize_url: String,
    pub opened_browser: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthStatusResponse {
    pub status: String,
    pub provider_auth: Option<ProviderAuthDto>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthCancelResponse {
    pub cancelled: bool,
}
