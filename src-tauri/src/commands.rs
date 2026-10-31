use crate::{
    error::{AppError, Result},
    metadata,
    models::*,
    queue::Queue,
    updater::{UpdateInfo, Updates},
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub queue: Arc<Queue>,
    pub updates: Updates,
    pub inspected: tokio::sync::Mutex<Option<Metadata>>,
}
#[tauri::command]
pub async fn snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    Ok(state.queue.snapshot().await)
}
#[tauri::command]
pub async fn inspect_url(state: State<'_, AppState>, url: String) -> Result<Metadata> {
    let result = metadata::inspect(&state.queue.engines, &url).await?;
    *state.inspected.lock().await = Some(result.clone());
    Ok(result)
}
#[tauri::command]
pub async fn enqueue(
    state: State<'_, AppState>,
    urls: Vec<String>,
    format: AudioFormat,
    quality: u16,
    playlist_confirmed: bool,
) -> Result<Vec<Job>> {
    let cache = state.inspected.lock().await;
    let inspected = cache
        .as_ref()
        .ok_or_else(|| AppError::new("metadata", "Inspect a URL before adding it to the queue."))?;
    if inspected.is_playlist && !playlist_confirmed {
        return Err(AppError::new(
            "playlist",
            "Confirm the playlist before adding entries.",
        ));
    }
    if urls.len() > 100 {
        return Err(AppError::new(
            "queue",
            "A playlist batch is limited to 100 entries.",
        ));
    }
    let mut unique = std::collections::HashSet::new();
    let items = urls
        .into_iter()
        .map(|url| {
            if !unique.insert(url.clone()) {
                return Err(AppError::new("queue", "Duplicate entry in batch."));
            }
            inspected
                .items
                .iter()
                .find(|m| m.url == url)
                .cloned()
                .ok_or_else(|| {
                    AppError::new(
                        "metadata",
                        "This URL was not in the inspected metadata. Inspect it again.",
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    state.queue.enqueue(items, format, quality).await
}
#[tauri::command]
pub async fn cancel_job(state: State<'_, AppState>, id: String) -> Result<()> {
    state.queue.cancel(&id).await
}
#[tauri::command]
pub async fn retry_job(state: State<'_, AppState>, id: String) -> Result<()> {
    state.queue.retry(&id).await
}
#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<()> {
    state.queue.clear().await
}
#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    state.queue.save_settings(settings).await
}
#[tauri::command]
pub async fn choose_folder(app: AppHandle) -> Result<Option<String>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose your AudioHarbor folder")
        .pick_folder(move |path| {
            let _ = sender.send(path);
        });
    let chosen = receiver
        .await
        .map_err(|_| AppError::new("dialog", "Folder picker closed unexpectedly."))?;
    chosen
        .map(|p| {
            p.into_path()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|_| AppError::new("path", "Choose a local directory."))
        })
        .transpose()
}
#[tauri::command]
pub async fn open_output(state: State<'_, AppState>, id: String, folder: bool) -> Result<()> {
    let snapshot = state.queue.snapshot().await;
    let job = snapshot
        .jobs
        .iter()
        .find(|j| j.id == id && j.status == Status::Completed)
        .ok_or_else(|| AppError::new("file", "No completed output is associated with this job."))?;
    let file = std::path::Path::new(
        job.output_file
            .as_deref()
            .ok_or_else(|| AppError::new("file", "No output filename recorded."))?,
    );
    let root = std::fs::canonicalize(&job.output_dir)?;
    let file = crate::paths::contained_file(file, &root)?;
    if !["mp3", "m4a", "flac", "wav"]
        .contains(&file.extension().and_then(|s| s.to_str()).unwrap_or(""))
    {
        return Err(AppError::new("file", "This file is not an audio output."));
    }
    let target = if folder {
        file.parent()
            .ok_or_else(|| AppError::new("file", "Parent folder unavailable."))?
            .to_path_buf()
    } else {
        file
    };
    tokio::task::spawn_blocking(move || open::that(target))
        .await
        .map_err(|_| AppError::new("file", "Could not open output."))??;
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    name: String,
    version: String,
    available: bool,
}
#[tauri::command]
pub async fn diagnostics(state: State<'_, AppState>) -> Result<Vec<Diagnostic>> {
    let e = &state.queue.engines;
    let mut results = Vec::new();
    for (name, path, arg) in [
        ("yt-dlp", &e.yt, "--version"),
        ("FFmpeg", &e.ffmpeg, "-version"),
        ("FFprobe", &e.ffprobe, "-version"),
        ("Deno", &e.deno, "--version"),
    ] {
        let result = crate::process::capture(path, &[arg.into()], 256 * 1024).await;
        results.push(match result {
            Ok(text) => Diagnostic {
                name: name.into(),
                version: text
                    .lines()
                    .next()
                    .unwrap_or("unknown")
                    .chars()
                    .take(160)
                    .collect(),
                available: true,
            },
            Err(_) => Diagnostic {
                name: name.into(),
                version: "Missing or unable to launch".into(),
                available: false,
            },
        });
    }
    Ok(results)
}
#[tauri::command]
pub async fn app_info(app: AppHandle) -> Result<serde_json::Value> {
    Ok(
        serde_json::json!({"version":app.package_info().version.to_string(),"updaterConfigured":crate::updater::configured(&app)}),
    )
}
#[tauri::command]
pub async fn check_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<UpdateInfo>> {
    state.updates.check(&app).await
}
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.updates.install(&app, state.queue.clone()).await
}
