use serde::{Deserialize, Serialize};

/// Summary of an available sprite that can be selected by the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: SpriteSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpriteSource {
    Builtin,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image: String,
    pub layout: SpriteLayout,
    pub scale: Option<f32>,
    pub frame_rate: Option<u32>,
    pub chroma_key: SpriteChromaKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteLayout {
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteChromaKey {
    pub target_color: [u8; 3],
    pub min_rb_over_g: u16,
    pub threshold: u16,
    pub softness: u16,
    pub spill_suppression: SpriteSpillSuppression,
    pub strip_dark_fringe: Option<bool>,
    pub pixel_art: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSpillSuppression {
    pub enabled: bool,
    pub threshold: u16,
    pub strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteManifestFile {
    pub manifest: SpriteManifest,
    pub image_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteImageValidation {
    pub image_width: u32,
    pub image_height: u32,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
    pub has_alpha: bool,
    pub background_mode: SpriteBackgroundMode,
    pub blank_frame_count: u32,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SpriteBackgroundMode {
    Transparent,
    FlatColor,
    Opaque,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteManifestValidation {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSpriteManifest {
    pub manifest: SpriteManifest,
    pub image_validation: SpriteImageValidation,
    pub manifest_validation: SpriteManifestValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSpriteManifestInput {
    pub image_path: String,
    pub name: String,
    pub description: Option<String>,
    pub columns: u32,
    pub rows: u32,
    pub scale: Option<f32>,
    pub frame_rate: Option<u32>,
    pub use_chroma_key: bool,
    pub pixel_art: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidateSpriteManifestInput {
    pub image_path: String,
    pub manifest: SpriteManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveCustomSpriteInput {
    pub image_path: String,
    pub manifest: SpriteManifest,
}
