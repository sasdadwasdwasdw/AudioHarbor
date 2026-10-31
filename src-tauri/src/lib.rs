mod commands;
mod download;
mod error;
mod metadata;
mod models;
mod paths;
mod process;
mod queue;
#[cfg(test)]
mod smoke_tests;
mod storage;
mod updater;
use tauri::Manager;
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("audioharbor_lib=info"),
    )
    .init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let storage = storage::Storage::open(&data.join("audioharbor.sqlite3"))?;
            let default_folder = app
                .path()
                .download_dir()?
                .join("AudioHarbor")
                .to_string_lossy()
                .into_owned();
            let settings = storage.settings(models::Settings::defaults(default_folder))?;
            settings.validate()?;
            let jobs = storage.jobs()?;
            storage.save_jobs(&jobs)?;
            let queue = queue::Queue::new(
                app.handle().clone(),
                storage,
                settings,
                jobs,
                process::Engines::resolve()?,
            );
            queue.start();
            app.manage(commands::AppState {
                queue,
                updates: updater::Updates::default(),
                inspected: tokio::sync::Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::inspect_url,
            commands::enqueue,
            commands::cancel_job,
            commands::retry_job,
            commands::clear_history,
            commands::save_settings,
            commands::choose_folder,
            commands::open_output,
            commands::diagnostics,
            commands::app_info,
            commands::check_updates,
            commands::install_update
        ])
        .run(tauri::generate_context!())
        .expect("AudioHarbor could not start");
}
