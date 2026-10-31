use crate::{
    error::{AppError, Result},
    queue::Queue,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

pub struct Updates {
    pub pending: Mutex<Option<Update>>,
    operation: Mutex<()>,
}
impl Default for Updates {
    fn default() -> Self {
        Self {
            pending: Mutex::new(None),
            operation: Mutex::new(()),
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
    phase: String,
}
pub fn configured(app: &AppHandle) -> bool {
    app.config().plugins.0.get("updater").is_some_and(|v| {
        let key = v.get("pubkey").and_then(|v| v.as_str()).unwrap_or("");
        !key.is_empty()
            && !key.contains("REPLACE_")
            && !v.to_string().contains("YOUR_GITHUB_")
            && !v.to_string().contains("YOUR_REPOSITORY")
    })
}
impl Updates {
    pub async fn check(&self, app: &AppHandle) -> Result<Option<UpdateInfo>> {
        let _operation = self.operation.try_lock().map_err(|_| {
            AppError::new("update_busy", "An update operation is already in progress.")
        })?;
        if !configured(app) {
            return Err(AppError::new("update_config","The distributor must configure a GitHub repository and Tauri public signing key before updates can be checked."));
        }
        let updater = app
            .updater_builder()
            .timeout(std::time::Duration::from_secs(30))
            .version_comparator(|current, release| release.version > current)
            .build()
            .map_err(|e| AppError::new("update", crate::error::redact(&e.to_string())))?;
        let update = updater.check().await.map_err(|e| {
            AppError::new(
                "update",
                format!(
                    "Could not check for updates: {}",
                    crate::error::redact(&e.to_string())
                ),
            )
        })?;
        let info = update.as_ref().map(|u| UpdateInfo {
            version: u.version.clone(),
            notes: u
                .body
                .clone()
                .unwrap_or_else(|| "No release notes provided.".into()),
        });
        *self.pending.lock().await = update;
        Ok(info)
    }
    pub async fn install(&self, app: &AppHandle, queue: Arc<Queue>) -> Result<()> {
        let _operation = self.operation.try_lock().map_err(|_| {
            AppError::new("update_busy", "An update operation is already in progress.")
        })?;
        let pending = self.pending.lock().await;
        let update = pending
            .as_ref()
            .ok_or_else(|| AppError::new("update", "Check for an available update first."))?;
        {
            let mut inner = queue.inner.lock().await;
            if !inner.active.is_empty() {
                return Err(AppError::new(
                    "downloads_active",
                    "Wait until active downloads finish before installing this update.",
                ));
            }
            inner.maintenance = true;
        }
        let mut downloaded = 0u64;
        let progress_app = app.clone();
        let finished_app = app.clone();
        let result = update
            .download_and_install(
                move |chunk, total| {
                    downloaded += chunk as u64;
                    let _ = progress_app.emit(
                        "update-progress",
                        UpdateProgress {
                            downloaded,
                            total,
                            phase: "downloading".into(),
                        },
                    );
                },
                move || {
                    let _ = finished_app.emit(
                        "update-progress",
                        UpdateProgress {
                            downloaded: 0,
                            total: None,
                            phase: "installing".into(),
                        },
                    );
                },
            )
            .await;
        // On Windows successful installer launch closes this process. Queued jobs are
        // already durable and will be marked interrupted/retryable after restart.
        queue.inner.lock().await.maintenance = false;
        queue.notify.notify_one();
        result.map_err(|e| {
            AppError::new(
                "update",
                format!(
                    "Update failed (including signature verification): {}",
                    crate::error::redact(&e.to_string())
                ),
            )
        })
    }
}
