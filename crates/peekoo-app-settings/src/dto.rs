use serde::Serialize;

/// Summary of an available sprite that can be selected by the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}
