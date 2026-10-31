use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
    Wav,
}
impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Flac => "flac",
            Self::Wav => "wav",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub output_dir: String,
    pub format: AudioFormat,
    pub quality: u16,
    pub concurrency: usize,
    pub theme: String,
    pub language: String,
    pub auto_update: bool,
}
impl Settings {
    pub fn defaults(dir: String) -> Self {
        Self {
            output_dir: dir,
            format: AudioFormat::Mp3,
            quality: 192,
            concurrency: 2,
            theme: "dark".into(),
            language: "en".into(),
            auto_update: true,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !(1..=4).contains(&self.concurrency)
            || ![128, 192, 256, 320].contains(&self.quality)
            || !["system", "dark", "light"].contains(&self.theme.as_str())
            || !["en", "th"].contains(&self.language.as_str())
        {
            return Err(AppError::new("settings", "Invalid settings values."));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: String,
    pub title: String,
    pub url: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub source: String,
    pub abr: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub title: String,
    pub is_playlist: bool,
    pub items: Vec<Media>,
    pub total: usize,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Queued,
    Downloading,
    Converting,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}
impl Status {
    pub fn active(&self) -> bool {
        matches!(self, Self::Downloading | Self::Converting)
    }
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled | Self::Interrupted)
    }
    pub fn can_transition(&self, to: &Self) -> bool {
        match self {
            Self::Queued => matches!(to, Self::Downloading | Self::Cancelled),
            Self::Downloading => matches!(
                to,
                Self::Converting | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Converting => matches!(
                to,
                Self::Completed | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Failed | Self::Cancelled | Self::Interrupted => *to == Self::Queued,
            Self::Completed => false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub percent: Option<f64>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub media: Media,
    pub status: Status,
    pub format: AudioFormat,
    pub quality: u16,
    pub output_dir: String,
    pub output_file: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub progress: Progress,
    pub error: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub jobs: Vec<Job>,
    pub settings: Settings,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transitions() {
        assert!(Status::Queued.can_transition(&Status::Downloading));
        assert!(!Status::Queued.can_transition(&Status::Completed));
        assert!(Status::Downloading.can_transition(&Status::Converting));
        assert!(Status::Converting.can_transition(&Status::Completed));
        assert!(Status::Cancelled.can_transition(&Status::Queued));
        assert!(!Status::Completed.can_transition(&Status::Queued));
    }
}
