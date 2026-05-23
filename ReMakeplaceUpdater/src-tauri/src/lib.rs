// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use anyhow::Context;
use base64::{engine::general_purpose, Engine as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;
use url::Url;
mod companion;
mod gallery;
use companion::ImportPayload;

mod config;
pub mod downloader;
mod error_handler;
mod extractor;
mod launcher;
mod retry_manager;
mod updater;

use config::{Config, ConfigManager, InstallationDetection, InstallationMode, InstallationStatus};
use downloader::{Downloader, ProgressInfo};
use error_handler::{ErrorHandler, ErrorInfo};
use extractor::Extractor;
use gallery as gallery_mod;
use gallery::GalleryItemDto;
use launcher::Launcher;
use updater::{UpdateInfo, UpdateManager};

// Application state to track current operations
#[derive(Default)]
pub struct AppState {
    pub current_config: Option<Config>,
    pub download_progress: ProgressInfo,
    pub is_updating: bool,
    pub is_downloading: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}

// Tauri Commands

#[tauri::command]
async fn load_config() -> Result<Config, String> {
    ConfigManager::load_config().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(config: Config) -> Result<(), String> {
    ConfigManager::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn validate_path(
    path: String,
    exe_name: String,
    mode: InstallationMode,
) -> Result<bool, String> {
    Ok(ConfigManager::validate_installation_path(
        &path, &exe_name, &mode,
    ))
}

#[tauri::command]
async fn validate_path_detailed(
    path: String,
    exe_name: String,
    mode: InstallationMode,
) -> Result<String, ErrorInfo> {
    match ConfigManager::validate_installation_path_detailed(&path, &exe_name, &mode) {
        Ok(()) => Ok("Path is valid".to_string()),
        Err(error_info) => Err(error_info),
    }
}

#[tauri::command]
async fn get_mode_description(mode: InstallationMode) -> Result<String, String> {
    Ok(ConfigManager::get_mode_description(&mode).to_string())
}

#[tauri::command]
async fn detect_installation_mode(
    path: String,
    exe_name: String,
) -> Result<InstallationMode, String> {
    Ok(ConfigManager::detect_installation_mode(&path, &exe_name))
}

#[tauri::command]
async fn detect_installation(
    path: String,
    exe_name: String,
) -> Result<InstallationDetection, String> {
    Ok(ConfigManager::detect_installation(&path, &exe_name))
}

#[tauri::command]
async fn set_version_to_latest(mut config: Config) -> Result<Config, String> {
    // Check for latest version and update config
    match UpdateManager::check_for_updates(&config).await {
        Ok(update_info) => {
            config.current_version = update_info.latest_version;
            config.last_check = chrono::Utc::now().to_rfc3339();
            ConfigManager::save_config(&config).map_err(|e| e.to_string())?;
            Ok(config)
        }
        Err(e) => Err(format!("Failed to fetch latest version: {}", e)),
    }
}

#[tauri::command]
async fn check_updates(config: Config) -> Result<UpdateInfo, String> {
    UpdateManager::check_for_updates(&config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_download(
    url: String,
    version: String,
    original_filename: String,
    expected_size: Option<u64>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err(
            "No supported download asset was found for the latest ReMakeplace release.".to_string(),
        );
    }

    // Check if a download is already in progress
    {
        let mut app_state = state.lock().await;
        if app_state.is_downloading {
            return Err("Download already in progress".to_string());
        }
        app_state.is_downloading = true;
    }

    let cache_dir = Downloader::get_cache_directory();
    let filepath = Downloader::get_cache_filepath(&cache_dir, &version, &original_filename);

    // Check if file already exists in cache and validate it
    let mut resume_download = false;
    if filepath.exists() {
        match Downloader::validate_cached_file(&filepath, expected_size) {
            Ok(true) => {
                println!("Found valid cached file: {}", filepath.display());
                // Reset download state since we're using cached file
                state.lock().await.is_downloading = false;
                let _ =
                    app_handle.emit("download-complete", &filepath.to_string_lossy().to_string());
                return Ok(filepath.to_string_lossy().to_string());
            }
            Ok(false) => {
                println!(
                    "Found invalid cached file, will attempt to resume: {}",
                    filepath.display()
                );
                resume_download = true;
            }
            Err(e) => {
                println!(
                    "Error validating cached file: {}, removing and redownloading",
                    e
                );
                if let Err(remove_err) = std::fs::remove_file(&filepath) {
                    println!(
                        "Warning: Failed to remove invalid cache file: {}",
                        remove_err
                    );
                    // Continue with download anyway
                }
            }
        }
    }

    let app_handle_progress = app_handle.clone();
    let app_handle_complete = app_handle.clone();
    let app_handle_error = app_handle;
    let filepath_clone = filepath.clone();
    let state_clone = state.inner().clone();

    tokio::spawn(async move {
        let progress_callback = move |progress: ProgressInfo| {
            let _ = app_handle_progress.emit("download-progress", &progress);
        };

        let download_result = Downloader::download_file_with_resume(
            &url,
            &filepath_clone,
            resume_download,
            progress_callback,
        )
        .await;

        // Always reset download state when done
        state_clone.lock().await.is_downloading = false;

        match download_result {
            Ok(()) => {
                // Validate the completed download
                match Downloader::validate_cached_file(&filepath_clone, expected_size) {
                    Ok(true) => {
                        let _ = app_handle_complete.emit(
                            "download-complete",
                            &filepath_clone.to_string_lossy().to_string(),
                        );
                    }
                    Ok(false) => {
                        // Remove invalid file
                        let _ = std::fs::remove_file(&filepath_clone);
                        let error_info = ErrorHandler::categorize_error(&anyhow::anyhow!(
                            "Downloaded file failed validation"
                        ));
                        let _ = app_handle_error.emit("download-error", &error_info);
                    }
                    Err(e) => {
                        let error_info = ErrorHandler::categorize_error(&e);
                        let _ = app_handle_error.emit("download-error", &error_info);
                    }
                }
            }
            Err(e) => {
                let error_info = ErrorHandler::categorize_error(&e);
                let _ = app_handle_error.emit("download-error", &error_info);
            }
        }
    });

    Ok(filepath.to_string_lossy().to_string())
}

#[tauri::command]
async fn install_update(
    archive_path: String,
    config: Config,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let archive_path = PathBuf::from(archive_path);
    let installation_path = PathBuf::from(&config.installation_path);

    tokio::spawn(async move {
        let _ = app_handle.emit("status-update", "Preparing extraction...");

        let staging_dir = create_temp_dir("staging");
        let backup_dir = create_temp_dir("backup");

        let archive_size = std::fs::metadata(&archive_path)
            .map(|metadata| {
                let size_gb = metadata.len() as f64 / 1_073_741_824.0;
                format!("{:.1} GB", size_gb)
            })
            .unwrap_or_else(|_| "large".to_string());
        let _ = app_handle.emit(
            "status-update",
            format!(
                "Extracting the {} release archive. This can take a few minutes.",
                archive_size
            ),
        );
        let extract_progress_handle = app_handle.clone();
        let extract_progress = Arc::new(move |message: String| {
            let _ = extract_progress_handle.emit("status-update", message);
        });
        if let Err(e) =
            Extractor::extract_archive_with_progress(&archive_path, &staging_dir, extract_progress)
                .await
        {
            let _ = cleanup_dir(&staging_dir).await;
            let _ = app_handle.emit("error", &format!("Extraction failed: {}", e));
            return;
        }

        let _ = app_handle.emit("status-update", "Validating extracted files...");
        let install_root =
            match ConfigManager::find_installation_root(&staging_dir, &config.exe_path) {
                Some(root) => root,
                None => {
                    let _ = cleanup_dir(&staging_dir).await;
                    let message = format!(
                        "Extraction failed: the archive did not contain {} and MakePlace/Content.",
                        config.exe_path
                    );
                    let _ = app_handle.emit("error", &message);
                    return;
                }
            };

        if let Err(e) =
            ConfigManager::validate_installation_structure(&install_root, &config.exe_path)
        {
            let _ = cleanup_dir(&staging_dir).await;
            let _ = app_handle.emit(
                "error",
                &format!("Extracted files failed validation: {}", e),
            );
            return;
        }

        // Only backup user data if this is an update or repair (not fresh install)
        if config.installation_mode == InstallationMode::Update {
            let _ = app_handle.emit("status-update", "Backing up user data...");
            if let Err(e) =
                backup_user_data(&installation_path, &backup_dir, &config.preserve_folders).await
            {
                let _ = cleanup_dir(&staging_dir).await;
                let _ = cleanup_dir(&backup_dir).await;
                let _ = app_handle.emit("error", &format!("Backup failed: {}", e));
                return;
            }
        }

        let _ = app_handle.emit("status-update", "Installing validated files...");
        if let Err(e) = copy_dir_all(&install_root, &installation_path) {
            if config.installation_mode == InstallationMode::Update {
                let _ =
                    restore_user_data(&installation_path, &backup_dir, &config.preserve_folders)
                        .await;
            }
            let _ = cleanup_dir(&staging_dir).await;
            let _ = cleanup_dir(&backup_dir).await;
            let _ = app_handle.emit("error", &format!("Failed to install files: {}", e));
            return;
        }

        if config.installation_mode == InstallationMode::Update {
            let _ = app_handle.emit("status-update", "Restoring user data...");
            if let Err(e) =
                restore_user_data(&installation_path, &backup_dir, &config.preserve_folders).await
            {
                let _ = cleanup_dir(&staging_dir).await;
                let _ = cleanup_dir(&backup_dir).await;
                let _ = app_handle.emit("error", &format!("Failed to restore user data: {}", e));
                return;
            }
        }

        if let Err(e) =
            ConfigManager::validate_installation_structure(&installation_path, &config.exe_path)
        {
            let _ = cleanup_dir(&staging_dir).await;
            let _ = cleanup_dir(&backup_dir).await;
            let _ = app_handle.emit(
                "error",
                &format!("Installed files failed validation: {}", e),
            );
            return;
        }

        // Update config with new version
        let mut updated_config = config.clone();
        if let Ok(update_info) = UpdateManager::check_for_updates(&config).await {
            updated_config.current_version = update_info.latest_version;
        }
        updated_config.installation_mode = InstallationMode::Update;
        updated_config.last_check = chrono::Utc::now().to_rfc3339();

        if let Err(e) = ConfigManager::save_config(&updated_config) {
            let _ = app_handle.emit("error", &format!("Failed to update config: {}", e));
            return;
        }

        // Clean up
        let _ = app_handle.emit("status-update", "Cleaning up...");
        let cache_dir = Downloader::get_cache_directory();
        let _ = Downloader::manage_cache(&cache_dir, false);
        let _ = cleanup_dir(&staging_dir).await;
        let _ = cleanup_dir(&backup_dir).await;

        let _ = app_handle.emit("status-update", "Update completed successfully!");
        let _ = app_handle.emit("update-complete", ());
    });

    Ok(())
}

#[tauri::command]
async fn launch_game(config: Config) -> Result<(), String> {
    let installation_path = PathBuf::from(&config.installation_path);

    Launcher::launch_game(&installation_path, &config.exe_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn browse_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);

    app.dialog().file().pick_folder(move |result| {
        let _ = sender.try_send(result);
    });

    if let Some(result) = receiver.recv().await {
        match result {
            Some(path) => Ok(Some(path.to_string())),
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn clear_cache() -> Result<(), String> {
    let cache_dir = Downloader::get_cache_directory();

    if cache_dir.exists() {
        Downloader::manage_cache(&cache_dir, false)
            .map_err(|e| format!("Failed to clear cache: {}", e))?;

        println!("Cache cleared successfully");
        Ok(())
    } else {
        Ok(()) // No cache to clear
    }
}

#[tauri::command]
async fn get_cache_path(version: String, original_filename: String) -> Result<String, String> {
    let cache_dir = Downloader::get_cache_directory();
    let filepath = Downloader::get_cache_filepath(&cache_dir, &version, &original_filename);
    Ok(filepath.to_string_lossy().to_string())
}

// Helper functions for data preservation

async fn backup_user_data(
    installation_path: &Path,
    backup_dir: &Path,
    preserve_folders: &[String],
) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(&backup_dir)?;

    for folder in preserve_folders {
        let source = installation_path.join(folder);
        if source.exists() {
            let dest = backup_dir.join(folder);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            copy_dir_all(&source, &dest)?;
        }
    }

    // Also backup config.json if it exists in installation directory
    let config_source = installation_path.join("config.json");
    if config_source.exists() {
        let config_dest = backup_dir.join("config.json");
        std::fs::copy(&config_source, &config_dest)?;
        println!(
            "Backed up MakePlace config.json from: {}",
            config_source.display()
        );
    }

    Ok(())
}

async fn restore_user_data(
    installation_path: &Path,
    backup_dir: &Path,
    preserve_folders: &[String],
) -> Result<(), anyhow::Error> {
    if !backup_dir.exists() {
        return Ok(()); // Nothing to restore
    }

    for folder in preserve_folders {
        let source = backup_dir.join(folder);
        let dest = installation_path.join(folder);

        if source.exists() {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            copy_dir_all(&source, &dest)?;
        }
    }

    // Smart restore config.json with merging
    let config_source = backup_dir.join("config.json");
    let config_dest = installation_path.join("config.json");

    if config_source.exists() {
        if let Err(e) = merge_config_files(&config_source, &config_dest).await {
            println!("Config merge failed, falling back to simple restore: {}", e);
            // Fallback to simple copy if merge fails
            std::fs::copy(&config_source, &config_dest)?;
        }
        println!(
            "Restored MakePlace config.json to: {}",
            config_dest.display()
        );
    }

    Ok(())
}

/// Smart config.json merging that preserves user settings while adding new options
async fn merge_config_files(backup_config: &Path, new_config: &Path) -> Result<(), anyhow::Error> {
    // Read the backed up (user) config
    let user_config_content =
        std::fs::read_to_string(backup_config).context("Failed to read user config.json")?;
    let mut user_config: serde_json::Value =
        serde_json::from_str(&user_config_content).context("Failed to parse user config.json")?;

    // Read the new (from update) config if it exists
    if new_config.exists() {
        let new_config_content =
            std::fs::read_to_string(new_config).context("Failed to read new config.json")?;
        let new_config_json: serde_json::Value =
            serde_json::from_str(&new_config_content).context("Failed to parse new config.json")?;

        // Merge: Add new keys from the update, preserve existing user values
        if let (Some(user_obj), Some(new_obj)) =
            (user_config.as_object_mut(), new_config_json.as_object())
        {
            for (key, new_value) in new_obj {
                if !user_obj.contains_key(key) {
                    // Add new option that didn't exist in user config
                    user_obj.insert(key.clone(), new_value.clone());
                    println!("Added new config option: {} = {}", key, new_value);
                }
                // Keep existing user values for all other keys
            }
        }
    }

    // Write the merged config back
    let merged_content =
        serde_json::to_string_pretty(&user_config).context("Failed to serialize merged config")?;
    std::fs::write(new_config, merged_content).context("Failed to write merged config.json")?;

    println!("Successfully merged config.json - preserved user settings and added new options");
    Ok(())
}

async fn cleanup_dir(dir: &Path) -> Result<(), anyhow::Error> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "remakeplace-autoupdater-{}-{}",
        prefix,
        uuid::Uuid::new_v4()
    ))
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            let source = entry.path();
            let target = dst.join(entry.file_name());
            std::fs::copy(&source, &target)?;
            if let Ok(metadata) = std::fs::metadata(&source) {
                let _ = std::fs::set_permissions(&target, metadata.permissions());
            }
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        // Ensure working directory is the EXE directory to avoid System32 cwd when launched via protocol
        .setup(|_| {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let _ = std::env::set_current_dir(dir);
                }
            }
            Ok(())
        })
        // Single Instance should be the first plugin registered
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let _ = app.emit(
                "single-instance",
                &serde_json::json!({
                    "argv": args,
                    "cwd": cwd,
                }),
            );
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_focus();
                let _ = win.unminimize();
                let _ = win.show();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                // Ensure protocol registration for dev/portable builds on current binary
                if let Err(e) = _app.deep_link().register_all() {
                    // Non-fatal: deep link may still work if already registered
                    eprintln!("Deep link register_all failed: {}", e);
                }
            }
            // Allow loading file images via convertFileSrc on asset.localhost
            // Tauri v2 maps convertFileSrc to asset.localhost automatically; no extra config needed here.
            Ok(())
        })
        .manage(Arc::new(Mutex::new(app_state)))
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            validate_path,
            validate_path_detailed,
            detect_installation_mode,
            detect_installation,
            get_mode_description,
            set_version_to_latest,
            check_updates,
            start_download,
            install_update,
            launch_game,
            browse_folder,
            clear_cache,
            get_cache_path,
            open_config_folder,
            open_game_data_folder,
            open_url,
            handle_deep_link,
            list_gallery,
            reveal_path,
            delete_gallery_entry,
            get_image_data_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn handle_deep_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    // Parse makeplace://import?payload=...
    let parsed = Url::parse(&url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "makeplace" {
        return Err("Unsupported scheme".to_string());
    }
    let host = parsed.host_str().unwrap_or("");
    if host != "import" {
        return Err("Unsupported action".to_string());
    }
    let payload_q = parsed
        .query_pairs()
        .find(|(k, _)| k == "payload")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| "Missing payload".to_string())?;
    let decoded = general_purpose::STANDARD
        .decode(percent_decode(&payload_q).as_bytes())
        .map_err(|e| e.to_string())?;
    let json_str = String::from_utf8(decoded).map_err(|e| e.to_string())?;
    let payload: ImportPayload = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    if let Err(e) = gallery::init_db() {
        return Err(e.to_string());
    }
    let config = match ConfigManager::load_config() {
        Ok(c) => c,
        Err(e) => return Err(e.to_string()),
    };

    match companion::import_design(&config, payload).await {
        Ok((json_path, _image)) => {
            let _ = app
                .notification()
                .builder()
                .title("ReMakeplace Autoupdater")
                .body(format!("Design has been added ({}).", json_path))
                .show();
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

fn percent_decode(s: &str) -> String {
    match urlencoding::decode(s) {
        Ok(v) => v.into_owned(),
        Err(_) => s.to_string(),
    }
}

#[tauri::command]
async fn list_gallery() -> Result<Vec<GalleryItemDto>, String> {
    gallery::init_db().map_err(|e| e.to_string())?;
    gallery::list_entries().map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_gallery_entry(id: String) -> Result<(), String> {
    gallery::init_db().map_err(|e| e.to_string())?;
    gallery_mod::delete_entry(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_config_folder(app: tauri::AppHandle) -> Result<(), String> {
    let config_dir = ConfigManager::get_config_path()
        .parent()
        .ok_or_else(|| "Failed to resolve config directory".to_string())?
        .to_path_buf();
    let dir_str = config_dir
        .to_str()
        .ok_or_else(|| "Invalid directory path".to_string())?;
    app.opener()
        .open_path(dir_str, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_game_data_folder(
    app: tauri::AppHandle,
    config: Config,
    folder: String,
) -> Result<(), String> {
    let detection = ConfigManager::detect_installation(&config.installation_path, &config.exe_path);
    if detection.status != InstallationStatus::ExistingValid {
        return Err(
            "Custom and Save can only be opened after a valid ReMakeplace installation is detected."
                .to_string(),
        );
    }

    let folder_name = match folder.as_str() {
        "custom" => "Custom",
        "save" => "Save",
        _ => return Err("Unsupported folder type".to_string()),
    };
    let target = ConfigManager::resolve_game_data_folder(&config.installation_path, folder_name)
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let target_str = target
        .to_str()
        .ok_or_else(|| "Invalid folder path".to_string())?;
    app.opener()
        .open_path(target_str, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reveal_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    // Use shell plugin to reveal in OS file explorer when possible
    #[cfg(target_os = "windows")]
    {
        use tauri_plugin_shell::ShellExt;
        let shell = app.shell();
        let _ = shell.command("explorer").args(["/select,", &path]).spawn();
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_shell::ShellExt;
        let shell = app.shell();
        let _ = shell.command("open").args(["-R", &path]).spawn();
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        use tauri_plugin_shell::ShellExt;
        let shell = app.shell();
        // Try common file managers
        let _ = shell
            .command("xdg-open")
            .args([std::path::Path::new(&path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".")])
            .spawn();
        return Ok(());
    }
}

#[tauri::command]
async fn get_image_data_url(path: String) -> Result<String, String> {
    use std::fs;
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}
