use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize, Clone)]
#[error("{message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, AppError>;
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn engine(message: &str) -> Self {
        Self::new(
            "engine",
            if message.trim().is_empty() {
                "The bundled engine failed without error details. Retry or run engine diagnostics."
                    .into()
            } else {
                redact(message)
            },
        )
    }
}
pub fn redact(message: &str) -> String {
    message
        .split_whitespace()
        .take(100)
        .map(|s| {
            let lower = s.to_ascii_lowercase();
            if lower.contains("http:")
                || lower.contains("https:")
                || lower.contains("cookie")
                || lower.contains("token")
                || lower.contains("authorization")
            {
                "[redacted]"
            } else {
                s
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new("io", e.to_string())
    }
}
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        log::error!(
            "Database operation failed: {}",
            e.sqlite_error_code()
                .map_or("unknown".into(), |c| format!("{c:?}"))
        );
        Self::new("storage", "Could not read or save the local database.")
    }
}
impl From<serde_json::Error> for AppError {
    fn from(_: serde_json::Error) -> Self {
        Self::new("parse", "The engine returned an invalid response.")
    }
}
