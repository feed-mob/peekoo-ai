use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use base64::Engine;
use image::ImageReader;
use rusqlite::Connection;

use crate::dto::{
    GenerateSpriteManifestInput, GeneratedSpriteManifest, SaveCustomSpriteInput, SpriteBackgroundMode,
    SpriteChromaKey, SpriteImageValidation, SpriteInfo, SpriteLayout, SpriteManifest,
    SpriteManifestFile, SpriteManifestValidation, SpriteSource, SpriteSpillSuppression,
    ValidateSpriteManifestInput, ValidationIssue,
};
use crate::store::AppSettingsStore;

const SETTING_ACTIVE_SPRITE_ID: &str = "active_sprite_id";
const SETTING_THEME_MODE: &str = "theme_mode";
const SETTING_APP_LANGUAGE: &str = "app_language";
const SETTING_LOG_LEVEL: &str = "log_level";
const DEFAULT_SPRITE_ID: &str = "dark-cat";
const DEFAULT_THEME_MODE: &str = "system";
const DEFAULT_APP_LANGUAGE: &str = "en";
const MANIFEST_FILE_NAME: &str = "manifest.json";

struct BuiltinSprite {
    id: &'static str,
    name: &'static str,
    description: &'static str,
}

const BUILTIN_SPRITES: &[BuiltinSprite] = &[
    BuiltinSprite {
        id: "dark-cat",
        name: "Dark Cat",
        description: "Default dark-themed AI pet.",
    },
    BuiltinSprite {
        id: "cute-dog",
        name: "Cute Dog",
        description: "A cute alternative AI pet.",
    },
];

const DEFAULT_SPRITE_PROMPT: &str = r#"Create a desktop pet sprite sheet for Peekoo.

Requirements:
- PNG or JPG (PNG recommended to reduce compression artifacts)
- Background color must be pure magenta #ff00ff only: no gradients, shadows, textures, noise, or compression artifacts; no second background color pixels; the app will automatically key out the background as transparent
- Character should not include any background, scene, ground, wall, smoke, light spots, or similar environment elements; everything except the character itself must remain pure magenta #ff00ff
- Avoid using the background color #ff00ff or very similar magenta tones in the character, props, shadows, reflections, rim lights, or glow, otherwise they will be keyed out too
- Arrange in an 8 columns x 7 rows grid, each cell must be a square frame (frame width equals height)
- Frames must touch each other directly: no whitespace, gaps, padding, margins, separator lines, or grid lines
- Overall aspect ratio should be approximately 8:7
- Recommended output at 4K-level high resolution, for example 4096x3584 with 512x512 per frame; the program will scale proportionally to 1024x896 with 128x128 per frame
- Ensure high image quality: clear details, sharp edges, no blur, no aliasing, no compression artifacts
- Same row represents the same animation; left to right are consecutive looped frames
- Idle (row 1) should include gentle breathing and blinking, not a completely static pose
- Sleepy/Rest (row 6) should only show closed-eye breathing frames; do not include yawning because looping yawns look awkward
- Adjacent frames within the same row must transition smoothly: no skipped frames, no sudden large movement, pose jumps, expression jumps, zoom changes, or viewpoint changes
- Character position should stay consistent across frames, ideally center-aligned, and should not be cropped at the edges

Row meanings (top to bottom):
- Row 1: Idle/Peek - gentle breathing, peeking from the bottom of the screen with half body and eyes visible, occasional blinking
- Row 2: Happy/Celebrate - cheerful expression and celebration gestures, used when tasks are completed
- Row 3: Working/Focus - focused expression and working posture, used during pomodoro sessions
- Row 4: Thinking - thinking expression and posture, used during AI processing
- Row 5: Reminder - reminder expression and gestures, used for task deadlines, health reminders, and AI message notifications
- Row 6: Sleepy/Rest - drowsy expression with closed-eye breathing, no yawning
- Row 7: Dragging - being dragged state, showing either surprise or cooperative motion while moving

Character: <character name>
- Style: <character style>
- Character appearance: <character appearance>
- Requirements: keep lighting and color palette consistent across the whole sheet, keep the background pure magenta #ff00ff, and keep the overall sprite sheet style fully consistent
- Output: only output the sprite sheet image file
"#;

/// Application-level settings service for user preferences and sprite management.
pub struct AppSettingsService {
    store: AppSettingsStore,
    sprites_root: PathBuf,
}

impl AppSettingsService {
    pub fn with_conn(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        Self::with_conn_and_sprites_root(conn, peekoo_paths::peekoo_sprites_dir()?)
    }

    pub fn with_conn_and_sprites_root(
        conn: Arc<Mutex<Connection>>,
        sprites_root: PathBuf,
    ) -> Result<Self, String> {
        Ok(Self {
            store: AppSettingsStore::with_conn(conn),
            sprites_root,
        })
    }

    pub fn get_active_sprite_id(&self) -> Result<String, String> {
        Ok(self
            .store
            .get(SETTING_ACTIVE_SPRITE_ID)?
            .unwrap_or_else(|| DEFAULT_SPRITE_ID.to_string()))
    }

    pub fn set_active_sprite_id(&self, sprite_id: &str) -> Result<(), String> {
        if !self.sprite_exists(sprite_id) {
            return Err(format!("Unknown sprite: {sprite_id}"));
        }
        self.store.set(SETTING_ACTIVE_SPRITE_ID, sprite_id)
    }

    pub fn get_theme_mode(&self) -> Result<String, String> {
        Ok(self
            .store
            .get(SETTING_THEME_MODE)?
            .unwrap_or_else(|| DEFAULT_THEME_MODE.to_string()))
    }

    pub fn set_theme_mode(&self, mode: &str) -> Result<(), String> {
        match mode {
            "light" | "dark" | "system" => self.store.set(SETTING_THEME_MODE, mode),
            _ => Err(format!("Invalid theme mode: {mode}")),
        }
    }

    pub fn get_app_language(&self) -> Result<String, String> {
        Ok(self
            .store
            .get(SETTING_APP_LANGUAGE)?
            .unwrap_or_else(|| DEFAULT_APP_LANGUAGE.to_string()))
    }

    pub fn set_app_language(&self, language: &str) -> Result<(), String> {
        match language {
            "en" | "zh-CN" | "zh-TW" | "ja" | "es" | "fr" => {
                self.store.set(SETTING_APP_LANGUAGE, language)
            }
            _ => Err(format!("Invalid app language: {language}")),
        }
    }

    pub fn list_sprites(&self) -> Vec<SpriteInfo> {
        let mut sprites: Vec<SpriteInfo> = BUILTIN_SPRITES
            .iter()
            .map(|sprite| SpriteInfo {
                id: sprite.id.to_string(),
                name: sprite.name.to_string(),
                description: sprite.description.to_string(),
                source: SpriteSource::Builtin,
            })
            .collect();

        sprites.extend(self.list_custom_sprites());
        sprites
    }

    pub fn get_all(&self) -> Result<HashMap<String, String>, String> {
        self.store.get_all()
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.store.get(key)
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        if key == SETTING_ACTIVE_SPRITE_ID {
            return self.set_active_sprite_id(value);
        }
        if key == SETTING_THEME_MODE {
            return self.set_theme_mode(value);
        }
        if key == SETTING_APP_LANGUAGE {
            return self.set_app_language(value);
        }
        if key == SETTING_LOG_LEVEL {
            return match value {
                "error" | "warn" | "info" | "debug" | "trace" => self.store.set(SETTING_LOG_LEVEL, value),
                _ => Err(format!("Invalid log level: {value}")),
            };
        }
        self.store.set(key, value)
    }

    pub fn get_sprite_prompt(&self) -> &'static str {
        DEFAULT_SPRITE_PROMPT
    }

    pub fn get_sprite_image_data_url(&self, image_path: &str) -> Result<String, String> {
        let path = Path::new(image_path);
        let bytes = fs::read(path).map_err(|e| format!("Failed to read sprite image: {e}"))?;
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        let mime_type = match extension.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            "jpg" | "jpeg" => "image/jpeg",
            _ => {
                match image::guess_format(&bytes).ok() {
                    Some(image::ImageFormat::Png) => "image/png",
                    Some(image::ImageFormat::WebP) => "image/webp",
                    Some(image::ImageFormat::Jpeg) => "image/jpeg",
                    _ => return Err(format!("Unsupported sprite image format: {extension}")),
                }
            }
        };

        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(format!("data:{mime_type};base64,{encoded}"))
    }

    pub fn get_sprite_manifest_template(&self) -> SpriteManifest {
        SpriteManifest {
            id: "my-custom-sprite".to_string(),
            name: "My Custom Sprite".to_string(),
            description: "A custom Peekoo sprite.".to_string(),
            image: "sprite.png".to_string(),
            layout: SpriteLayout {
                columns: 8,
                rows: 7,
            },
            scale: Some(0.35),
            frame_rate: Some(6),
            chroma_key: default_chroma_key(true, false),
        }
    }

    pub fn load_manifest_file(&self, manifest_path: &str) -> Result<SpriteManifest, String> {
        let contents = fs::read_to_string(manifest_path)
            .map_err(|e| format!("Failed to read manifest file: {e}"))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse manifest file: {e}"))
    }

    pub fn get_custom_sprite_manifest(&self, sprite_id: &str) -> Result<SpriteManifestFile, String> {
        let sprite_dir = self.custom_sprite_dir(sprite_id);
        if !sprite_dir.exists() {
            return Err(format!("Unknown custom sprite: {sprite_id}"));
        }

        let manifest = self.read_custom_manifest(sprite_id)?;
        let image_path = sprite_dir.join(&manifest.image);
        Ok(SpriteManifestFile {
            manifest,
            image_path: image_path.to_string_lossy().to_string(),
        })
    }

    pub fn generate_sprite_manifest_draft(
        &self,
        input: GenerateSpriteManifestInput,
    ) -> Result<GeneratedSpriteManifest, String> {
        let image_validation = self.validate_sprite_image(&input.image_path, input.columns, input.rows)?;
        let id = slugify_sprite_id(&input.name);
        let image_name = sanitize_file_name(
            Path::new(&input.image_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("sprite.png"),
        );

        let manifest = SpriteManifest {
            id,
            name: input.name.trim().to_string(),
            description: input.description.unwrap_or_else(|| "Custom Peekoo sprite".to_string()),
            image: image_name,
            layout: SpriteLayout {
                columns: input.columns,
                rows: input.rows,
            },
            scale: Some(input.scale.unwrap_or(0.35)),
            frame_rate: Some(input.frame_rate.unwrap_or(6)),
            chroma_key: default_chroma_key(input.use_chroma_key || !image_validation.has_alpha, input.pixel_art),
        };
        let manifest_validation = self.validate_manifest(&ValidateSpriteManifestInput {
            image_path: input.image_path,
            manifest: manifest.clone(),
        })?;

        Ok(GeneratedSpriteManifest {
            manifest,
            image_validation,
            manifest_validation,
        })
    }

    pub fn validate_manifest(
        &self,
        input: &ValidateSpriteManifestInput,
    ) -> Result<SpriteManifestValidation, String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if input.manifest.id.trim().is_empty() {
            errors.push(issue("id", "Sprite ID is required."));
        } else if slugify_sprite_id(&input.manifest.id) != input.manifest.id {
            errors.push(issue(
                "id",
                "Sprite ID must use lowercase letters, numbers, and hyphens only.",
            ));
        }

        if input.manifest.name.trim().is_empty() {
            errors.push(issue("name", "Sprite name is required."));
        }

        if Path::new(&input.manifest.image)
            .components()
            .count()
            != 1
        {
            errors.push(issue("image", "Manifest image must be a file name without directories."));
        }

        if input.manifest.layout.columns == 0 || input.manifest.layout.rows == 0 {
            errors.push(issue(
                "layout",
                "Layout columns and rows must both be greater than zero.",
            ));
        }

        if let Some(scale) = input.manifest.scale {
            if scale <= 0.0 {
                errors.push(issue("scale", "Scale must be greater than zero."));
            } else if scale > 1.0 {
                warnings.push(issue("scale", "Scale above 1.0 may appear oversized in the desktop window."));
            }
        }

        if let Some(frame_rate) = input.manifest.frame_rate {
            if frame_rate == 0 {
                errors.push(issue("frameRate", "Frame rate must be greater than zero."));
            } else if frame_rate > 12 {
                warnings.push(issue("frameRate", "Frame rates above 12 can feel too fast for desktop idle animations."));
            }
        }

        let image_validation = self.validate_sprite_image(
            &input.image_path,
            input.manifest.layout.columns,
            input.manifest.layout.rows,
        )?;
        errors.extend(image_validation.errors.iter().cloned());
        warnings.extend(image_validation.warnings.iter().cloned());

        Ok(SpriteManifestValidation { errors, warnings })
    }

    pub fn validate_sprite_image(
        &self,
        image_path: &str,
        columns: u32,
        rows: u32,
    ) -> Result<SpriteImageValidation, String> {
        if columns == 0 || rows == 0 {
            return Ok(SpriteImageValidation {
                image_width: 0,
                image_height: 0,
                frame_width: None,
                frame_height: None,
                has_alpha: false,
                background_mode: SpriteBackgroundMode::Opaque,
                blank_frame_count: 0,
                errors: vec![issue("layout", "Columns and rows must be greater than zero.")],
                warnings: Vec::new(),
            });
        }

        let image_path_ref = Path::new(image_path);
        let extension = image_path_ref
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if !matches!(extension.as_str(), "png" | "webp" | "jpg" | "jpeg") {
            errors.push(issue(
                "image",
                "Only PNG, WebP, and JPEG sprite sheets are supported.",
            ));
        }

        let reader = ImageReader::open(image_path_ref)
            .map_err(|e| format!("Failed to open sprite image: {e}"))?;
        let image = reader
            .decode()
            .map_err(|e| format!("Failed to decode sprite image: {e}"))?;
        let rgba = image.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        if width == 0 || height == 0 {
            errors.push(issue("image", "Sprite image dimensions must be greater than zero."));
        }

        if width % columns != 0 {
            warnings.push(issue(
                "layout.columns",
                "Image width does not divide evenly by the number of columns. Peekoo will slice frames using proportional boundaries.",
            ));
        }

        if height % rows != 0 {
            warnings.push(issue(
                "layout.rows",
                "Image height does not divide evenly by the number of rows. Peekoo will slice frames using proportional boundaries.",
            ));
        }

        let frame_width = Some(((width as f32) / (columns as f32)).round() as u32);
        let frame_height = Some(((height as f32) / (rows as f32)).round() as u32);
        if let Some(frame_width) = frame_width
            && frame_width < 16
        {
            warnings.push(issue("image", "Frame width is very small and may render poorly."));
        }
        if let Some(frame_height) = frame_height
            && frame_height < 16
        {
            warnings.push(issue("image", "Frame height is very small and may render poorly."));
        }

        let has_alpha = rgba.pixels().any(|pixel| pixel.0[3] < 255);
        let corners = [
            rgba.get_pixel(0, 0).0,
            rgba.get_pixel(width.saturating_sub(1), 0).0,
            rgba.get_pixel(0, height.saturating_sub(1)).0,
            rgba.get_pixel(width.saturating_sub(1), height.saturating_sub(1)).0,
        ];
        let background_mode = if has_alpha {
            SpriteBackgroundMode::Transparent
        } else if corners.iter().all(|pixel| *pixel == corners[0]) {
            SpriteBackgroundMode::FlatColor
        } else {
            SpriteBackgroundMode::Opaque
        };

        if matches!(background_mode, SpriteBackgroundMode::Opaque) {
            warnings.push(issue(
                "image",
                "No transparency or flat chroma-key background was detected. Chroma key tuning may be required.",
            ));
        }

        let blank_frame_count = match (frame_width, frame_height) {
            (Some(frame_width), Some(frame_height)) => {
                count_blank_frames(&rgba, frame_width, frame_height, background_mode)
            }
            _ => 0,
        };
        if blank_frame_count > 0 {
            warnings.push(issue(
                "image",
                &format!("Detected {blank_frame_count} blank frame(s) in the sprite sheet."),
            ));
        }

        Ok(SpriteImageValidation {
            image_width: width,
            image_height: height,
            frame_width,
            frame_height,
            has_alpha,
            background_mode,
            blank_frame_count,
            errors,
            warnings,
        })
    }

    pub fn save_custom_sprite(&self, input: SaveCustomSpriteInput) -> Result<SpriteInfo, String> {
        let manifest_validation = self.validate_manifest(&ValidateSpriteManifestInput {
            image_path: input.image_path.clone(),
            manifest: input.manifest.clone(),
        })?;
        if !manifest_validation.errors.is_empty() {
            return Err(format!(
                "Sprite validation failed: {}",
                manifest_validation
                    .errors
                    .iter()
                    .map(|issue| issue.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }

        if BUILTIN_SPRITES.iter().any(|sprite| sprite.id == input.manifest.id) {
            return Err("Custom sprite ID conflicts with a built-in sprite.".to_string());
        }

        let sprite_dir = self.custom_sprite_dir(&input.manifest.id);
        if sprite_dir.exists() {
            return Err(format!("Custom sprite '{}' already exists.", input.manifest.id));
        }

        fs::create_dir_all(&sprite_dir).map_err(|e| format!("Failed to create sprite directory: {e}"))?;
        let source_path = Path::new(&input.image_path);
        let copied_image_name = sanitize_file_name(
            source_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("sprite.png"),
        );
        let destination_image_path = sprite_dir.join(&copied_image_name);
        fs::copy(source_path, &destination_image_path)
            .map_err(|e| format!("Failed to copy sprite image: {e}"))?;

        let mut manifest = input.manifest;
        manifest.image = copied_image_name;
        self.write_custom_manifest(&manifest)?;

        Ok(SpriteInfo {
            id: manifest.id,
            name: manifest.name,
            description: manifest.description,
            source: SpriteSource::Custom,
        })
    }

    pub fn delete_custom_sprite(&self, sprite_id: &str) -> Result<(), String> {
        let sprite_dir = self.custom_sprite_dir(sprite_id);
        if !sprite_dir.exists() {
            return Err(format!("Unknown custom sprite: {sprite_id}"));
        }
        fs::remove_dir_all(&sprite_dir).map_err(|e| format!("Failed to delete custom sprite: {e}"))?;
        if self.get_active_sprite_id()? == sprite_id {
            self.store.set(SETTING_ACTIVE_SPRITE_ID, DEFAULT_SPRITE_ID)?;
        }
        Ok(())
    }

    fn sprite_exists(&self, sprite_id: &str) -> bool {
        BUILTIN_SPRITES.iter().any(|sprite| sprite.id == sprite_id)
            || self.custom_sprite_dir(sprite_id).join(MANIFEST_FILE_NAME).exists()
    }

    fn list_custom_sprites(&self) -> Vec<SpriteInfo> {
        let Ok(entries) = fs::read_dir(&self.sprites_root) else {
            return Vec::new();
        };

        let mut sprites = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || path.file_name().and_then(|name| name.to_str()) == Some("_drafts") {
                continue;
            }
            let manifest_path = path.join(MANIFEST_FILE_NAME);
            if !manifest_path.exists() {
                continue;
            }
            let Ok(contents) = fs::read_to_string(&manifest_path) else {
                continue;
            };
            let Ok(manifest) = serde_json::from_str::<SpriteManifest>(&contents) else {
                continue;
            };
            sprites.push(SpriteInfo {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                description: manifest.description.clone(),
                source: SpriteSource::Custom,
            });
        }
        sprites.sort_by(|left, right| left.name.cmp(&right.name));
        sprites
    }

    fn read_custom_manifest(&self, sprite_id: &str) -> Result<SpriteManifest, String> {
        let manifest_path = self.custom_sprite_dir(sprite_id).join(MANIFEST_FILE_NAME);
        let contents = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read custom sprite manifest: {e}"))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse custom sprite manifest: {e}"))
    }

    fn write_custom_manifest(&self, manifest: &SpriteManifest) -> Result<(), String> {
        let sprite_dir = self.custom_sprite_dir(&manifest.id);
        fs::create_dir_all(&sprite_dir).map_err(|e| format!("Failed to create sprite directory: {e}"))?;
        let manifest_path = sprite_dir.join(MANIFEST_FILE_NAME);
        let content = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("Failed to serialize sprite manifest: {e}"))?;
        fs::write(&manifest_path, content).map_err(|e| format!("Failed to write sprite manifest: {e}"))
    }

    fn custom_sprite_dir(&self, sprite_id: &str) -> PathBuf {
        self.sprites_root.join(sprite_id)
    }
}

fn sanitize_file_name(file_name: &str) -> String {
    let path = Path::new(file_name);
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| {
            value
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty());

    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| {
            value
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                .collect::<String>()
        })
        .unwrap_or_default();

    let mut normalized_stem = stem
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase();

    if normalized_stem.is_empty() {
        normalized_stem = "sprite".to_string();
    }

    let normalized_ext = ext.unwrap_or_else(|| "png".to_string());
    format!("{normalized_stem}.{normalized_ext}")
}

fn slugify_sprite_id(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in name.trim().chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            slug.push(normalized);
            last_was_dash = false;
            continue;
        }
        if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn default_chroma_key(use_chroma_key: bool, pixel_art: bool) -> SpriteChromaKey {
    let target_color = if use_chroma_key { [255, 0, 255] } else { [0, 0, 0] };
    SpriteChromaKey {
        target_color,
        min_rb_over_g: 32,
        threshold: if use_chroma_key { 110 } else { 0 },
        softness: if use_chroma_key { 80 } else { 0 },
        spill_suppression: SpriteSpillSuppression {
            enabled: use_chroma_key,
            threshold: 260,
            strength: 0.9,
        },
        strip_dark_fringe: Some(use_chroma_key),
        pixel_art: Some(pixel_art),
    }
}

fn count_blank_frames(
    rgba: &image::RgbaImage,
    frame_width: u32,
    frame_height: u32,
    background_mode: SpriteBackgroundMode,
) -> u32 {
    let background = rgba.get_pixel(0, 0).0;
    let columns = rgba.width() / frame_width;
    let rows = rgba.height() / frame_height;
    let mut blank = 0;

    for row in 0..rows {
        for column in 0..columns {
            let mut has_content = false;
            'frame: for y in (row * frame_height)..((row + 1) * frame_height) {
                for x in (column * frame_width)..((column + 1) * frame_width) {
                    let pixel = rgba.get_pixel(x, y).0;
                    let is_background = match background_mode {
                        SpriteBackgroundMode::Transparent => pixel[3] == 0,
                        SpriteBackgroundMode::FlatColor => pixel == background,
                        SpriteBackgroundMode::Opaque => false,
                    };
                    if !is_background {
                        has_content = true;
                        break 'frame;
                    }
                }
            }
            if !has_content {
                blank += 1;
            }
        }
    }

    blank
}

fn issue(field: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        field: field.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use tempfile::TempDir;

    fn test_service() -> (AppSettingsService, TempDir) {
        let conn = peekoo_persistence_sqlite::setup_test_db();
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = AppSettingsService::with_conn_and_sprites_root(
            Arc::new(Mutex::new(conn)),
            temp_dir.path().join("sprites"),
        )
        .expect("service");
        (service, temp_dir)
    }

    fn write_test_sprite(path: &Path) {
        let image = ImageBuffer::from_fn(160, 140, |x, y| {
            if x % 20 == 0 || y % 20 == 0 {
                Rgba([255_u8, 0_u8, 255_u8, 255_u8])
            } else {
                Rgba([255_u8, 200_u8, 0_u8, 255_u8])
            }
        });
        image.save(path).expect("save sprite");
    }

    #[test]
    fn default_sprite_is_dark_cat() {
        let (svc, _) = test_service();
        assert_eq!(svc.get_active_sprite_id().unwrap(), "dark-cat");
    }

    #[test]
    fn set_valid_sprite_succeeds_for_builtin() {
        let (svc, _) = test_service();
        svc.set_active_sprite_id("cute-dog").unwrap();
        assert_eq!(svc.get_active_sprite_id().unwrap(), "cute-dog");
    }

    #[test]
    fn list_sprites_returns_builtins() {
        let (svc, _) = test_service();
        let sprites = svc.list_sprites();
        assert_eq!(sprites.len(), 2);
        assert_eq!(sprites[0].source, SpriteSource::Builtin);
    }

    #[test]
    fn generated_manifest_uses_slugified_name() {
        let (svc, temp_dir) = test_service();
        let image_path = temp_dir.path().join("dog.png");
        write_test_sprite(&image_path);

        let generated = svc
            .generate_sprite_manifest_draft(GenerateSpriteManifestInput {
                image_path: image_path.to_string_lossy().to_string(),
                name: "My Dog Buddy".to_string(),
                description: None,
                columns: 8,
                rows: 7,
                scale: None,
                frame_rate: None,
                use_chroma_key: true,
                pixel_art: false,
            })
            .unwrap();

        assert_eq!(generated.manifest.id, "my-dog-buddy");
        assert!(generated.image_validation.errors.is_empty());
    }

    #[test]
    fn save_custom_sprite_adds_to_catalog() {
        let (svc, temp_dir) = test_service();
        let image_path = temp_dir.path().join("dog.png");
        write_test_sprite(&image_path);

        let generated = svc
            .generate_sprite_manifest_draft(GenerateSpriteManifestInput {
                image_path: image_path.to_string_lossy().to_string(),
                name: "Buddy".to_string(),
                description: Some("Friendly pet".to_string()),
                columns: 8,
                rows: 7,
                scale: Some(0.3),
                frame_rate: Some(6),
                use_chroma_key: true,
                pixel_art: false,
            })
            .unwrap();

        svc.save_custom_sprite(SaveCustomSpriteInput {
            image_path: image_path.to_string_lossy().to_string(),
            manifest: generated.manifest,
        })
        .unwrap();

        let sprites = svc.list_sprites();
        assert_eq!(sprites.len(), 3);
        assert!(sprites.iter().any(|sprite| sprite.id == "buddy" && sprite.source == SpriteSource::Custom));
    }

    #[test]
    fn validation_warns_but_does_not_fail_for_non_even_grid_division() {
        let (svc, temp_dir) = test_service();
        let image_path = temp_dir.path().join("dog.png");
        let image = ImageBuffer::from_fn(2208, 1920, |_, _| Rgba([255_u8, 0_u8, 255_u8, 255_u8]));
        image.save(&image_path).expect("save sprite");

        let result = svc
            .generate_sprite_manifest_draft(GenerateSpriteManifestInput {
                image_path: image_path.to_string_lossy().to_string(),
                name: "Cute Dog".to_string(),
                description: None,
                columns: 8,
                rows: 7,
                scale: None,
                frame_rate: None,
                use_chroma_key: true,
                pixel_art: false,
            })
            .unwrap();

        assert!(result.manifest_validation.errors.is_empty());
        assert!(result.manifest_validation.warnings.iter().any(|issue| issue.field == "layout.rows"));
    }

    #[test]
    fn delete_custom_sprite_falls_back_to_default() {
        let (svc, temp_dir) = test_service();
        let image_path = temp_dir.path().join("dog.png");
        write_test_sprite(&image_path);

        let generated = svc
            .generate_sprite_manifest_draft(GenerateSpriteManifestInput {
                image_path: image_path.to_string_lossy().to_string(),
                name: "Buddy".to_string(),
                description: None,
                columns: 8,
                rows: 7,
                scale: None,
                frame_rate: None,
                use_chroma_key: true,
                pixel_art: false,
            })
            .unwrap();

        svc.save_custom_sprite(SaveCustomSpriteInput {
            image_path: image_path.to_string_lossy().to_string(),
            manifest: generated.manifest,
        })
        .unwrap();
        svc.set_active_sprite_id("buddy").unwrap();

        svc.delete_custom_sprite("buddy").unwrap();

        assert_eq!(svc.get_active_sprite_id().unwrap(), "dark-cat");
    }

    #[test]
    fn sanitize_file_name_preserves_extension_for_non_ascii_names() {
        assert_eq!(sanitize_file_name("齐天大圣.png"), "sprite.png");
        assert_eq!(sanitize_file_name("悟空.webp"), "sprite.webp");
        assert_eq!(sanitize_file_name("my sprite.JPG"), "my-sprite.jpg");
    }
}
