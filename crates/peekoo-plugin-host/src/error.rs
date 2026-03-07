use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin not initialized: {0}")]
    NotInitialized(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Data provider not found: {0}")]
    DataProviderNotFound(String),

    #[error("Manifest parse error: {0}")]
    ManifestParse(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("WASM runtime error: {0}")]
    Runtime(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<extism::Error> for PluginError {
    fn from(e: extism::Error) -> Self {
        PluginError::Runtime(e.to_string())
    }
}
