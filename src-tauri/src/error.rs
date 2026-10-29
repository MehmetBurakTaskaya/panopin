use serde::{Serialize, Serializer};

/// Errors raised by Panopin's core. Messages never include clipboard content.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("window system error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("database schema version {found} is newer than this app supports ({supported})")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("unsupported image data: {0}")]
    BadImage(&'static str),
    #[error("invalid setting: {0}")]
    InvalidSetting(&'static str),
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;

// Commands hand errors to the webview as plain strings.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
