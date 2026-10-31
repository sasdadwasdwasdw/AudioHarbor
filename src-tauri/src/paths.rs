use crate::error::{AppError, Result};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

pub fn validate_url(input: &str) -> Result<String> {
    if input.len() > 8192 || input.chars().any(char::is_control) {
        return Err(AppError::new(
            "url",
            "Enter a valid public HTTP or HTTPS URL.",
        ));
    }
    let url = url::Url::parse(input.trim())
        .map_err(|_| AppError::new("url", "Enter a complete URL, including https://."))?;
    let host = url.host_str().unwrap_or("");
    if !["https", "http"].contains(&url.scheme())
        || host.is_empty()
        || !url.username().is_empty()
        || url.password().is_some()
        || host == "localhost"
        || host.ends_with(".localhost")
    {
        return Err(AppError::new(
            "url",
            "Use a public HTTP or HTTPS URL without embedded credentials.",
        ));
    }
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<std::net::IpAddr>() {
        let private = match ip {
            std::net::IpAddr::V4(v) => {
                v.is_private()
                    || v.is_loopback()
                    || v.is_link_local()
                    || v.is_unspecified()
                    || v.is_broadcast()
            }
            std::net::IpAddr::V6(v) => {
                v.is_loopback()
                    || v.is_unspecified()
                    || v.is_unique_local()
                    || v.is_unicast_link_local()
            }
        };
        if private {
            return Err(AppError::new(
                "url",
                "Local network addresses are not supported.",
            ));
        }
    }
    Ok(url.to_string())
}
pub fn validate_directory(input: &str) -> Result<PathBuf> {
    let path = Path::new(input);
    if !path.is_absolute()
        || input.starts_with("\\\\")
        || input.starts_with("//")
        || input.chars().any(char::is_control)
        || path.components().any(|c| matches!(c, Component::ParentDir))
    {
        return Err(AppError::new(
            "path",
            "Choose an absolute local folder without parent traversal or network prefixes.",
        ));
    }
    for component in path.components() {
        if let Component::Normal(v) = component {
            let name = v.to_string_lossy();
            let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            if name.contains([':', '*', '?', '"', '<', '>', '|'])
                || name.ends_with([' ', '.'])
                || [
                    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                    "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
                    "LPT8", "LPT9",
                ]
                .contains(&base.as_str())
            {
                return Err(AppError::new(
                    "path",
                    "This folder contains a Windows-reserved name or character.",
                ));
            }
        }
    }
    fs::create_dir_all(path)?;
    let canonical = fs::canonicalize(path)?;
    if canonical.parent().is_none() {
        return Err(AppError::new(
            "path",
            "Choose a folder rather than a drive root.",
        ));
    }
    let probe = canonical.join(format!(".audioharbor-write-{}", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)?;
    file.write_all(b"ok")?;
    drop(file);
    fs::remove_file(probe)?;
    Ok(canonical)
}
pub fn staging(dir: &str, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).map_err(|_| AppError::new("path", "Invalid job identifier."))?;
    let root = validate_directory(dir)?;
    let work = root.join(".audioharbor-work");
    fs::create_dir_all(&work)?;
    let work = fs::canonicalize(work)?;
    if !work.starts_with(&root) {
        return Err(AppError::new(
            "path",
            "Staging parent escapes the download folder.",
        ));
    }
    let stage = work.join(id);
    fs::create_dir_all(&stage)?;
    let stage = fs::canonicalize(stage)?;
    if !stage.starts_with(&root) {
        return Err(AppError::new(
            "path",
            "Staging folder escapes the download folder.",
        ));
    }
    Ok(stage)
}
pub fn contained_file(path: &Path, root: &Path) -> Result<PathBuf> {
    let file = fs::canonicalize(path)?;
    let root = fs::canonicalize(root)?;
    if !file.starts_with(root) || !file.is_file() {
        return Err(AppError::new(
            "path",
            "Engine output is outside its staging folder.",
        ));
    }
    Ok(file)
}
pub fn safe_name(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*%".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(90)
        .collect();
    let clean = clean.trim_matches([' ', '.']);
    if clean.is_empty() {
        "Audio".into()
    } else {
        clean.into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls() {
        for url in [
            "file:///C:/secret",
            "https://user:pass@example.com",
            "http://127.0.0.1",
            "--exec calc",
            "https://localhost",
        ] {
            assert!(validate_url(url).is_err());
        }
        assert!(validate_url("https://www.youtube.com/watch?v=abc").is_ok());
    }
    #[test]
    fn filenames() {
        assert_eq!(safe_name("เพลงไทย: demo?"), "เพลงไทย_ demo_");
        assert_eq!(safe_name(".."), "Audio");
    }
    #[test]
    fn traversal() {
        assert!(validate_directory("relative").is_err());
        assert!(validate_directory("C:\\foo\\..\\bar").is_err());
        assert!(validate_directory("\\\\server\\share").is_err());
        assert!(validate_directory("C:\\temp\\NUL.txt").is_err());
    }
    #[test]
    fn containment() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::NamedTempFile::new().unwrap();
        assert!(contained_file(other.path(), root.path()).is_err());
    }
}
