// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Emitter};
use anyhow::Context;

mod config;
mod updater;
mod downloader;
mod extractor;
mod launcher;

use config::{Config, ConfigManager};
use updater::{UpdateInfo, UpdateManager};
use downloader::{Downloader, ProgressInfo};
use extractor::Extractor;
use launcher::Launcher;

// Application state to track current operations
pub struct AppState {
    pub current_config: Arc<Mutex<Option<Config>>>,
    pub download_progress: Arc<Mutex<ProgressInfo>>,
    pub is_updating: Arc<Mutex<bool>>,
    pub is_downloading: Arc<Mutex<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_config: Arc::new(Mutex::new(None)),
            download_progress: Arc::new(Mutex::new(ProgressInfo::default())),
            is_updating: Arc::new(Mutex::new(false)),
            is_downloading: Arc::new(Mutex::new(false)),
        }
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
async fn validate_path(path: String, exe_name: String) -> Result<bool, String> {
    Ok(ConfigManager::validate_installation_path(&path, &exe_name))
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
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Check if a download is already in progress
    {
        let mut downloading = state.is_downloading.lock().await;
        if *downloading {
            return Err("Download already in progress".to_string());
        }
        *downloading = true;
    }

    let cache_dir = Downloader::get_cache_directory();
    let filepath = Downloader::get_cache_filepath(&cache_dir, &version, &original_filename);

    // Check if file already exists in cache and validate it
    let mut resume_download = false;
    if filepath.exists() {
        match Downloader::validate_cached_file(&filepath, None) {
            Ok(true) => {
                println!("Found valid cached file: {}", filepath.display());
                // Reset download state since we're using cached file
                *state.is_downloading.lock().await = false;
                return Ok(filepath.to_string_lossy().to_string());
            }
            Ok(false) => {
                println!("Found invalid cached file, will attempt to resume: {}", filepath.display());
                resume_download = true;
            }
            Err(e) => {
                println!("Error validating cached file: {}, removing and redownloading", e);
                if let Err(remove_err) = std::fs::remove_file(&filepath) {
                    println!("Warning: Failed to remove invalid cache file: {}", remove_err);
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

        let download_result = Downloader::download_file_with_resume(&url, &filepath_clone, resume_download, progress_callback).await;
        
        // Always reset download state when done
        *state_clone.is_downloading.lock().await = false;

        match download_result {
            Ok(()) => {
                // Validate the completed download
                match Downloader::validate_cached_file(&filepath_clone, None) {
                    Ok(true) => {
                        let _ = app_handle_complete.emit("download-complete", &filepath_clone.to_string_lossy().to_string());
                    }
                    Ok(false) => {
                        // Remove invalid file
                        let _ = std::fs::remove_file(&filepath_clone);
                        let _ = app_handle_error.emit("download-error", "Downloaded file failed validation");
                    }
                    Err(e) => {
                        let _ = app_handle_error.emit("download-error", &format!("Error validating downloaded file: {}", e));
                    }
                }
            }
            Err(e) => {
                let _ = app_handle_error.emit("download-error", &e.to_string());
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
        let _ = app_handle.emit("status-update", "Starting installation...");

        // Backup user data
        let _ = app_handle.emit("status-update", "Backing up user data...");
        if let Err(e) = backup_user_data(&installation_path, &config.preserve_folders).await {
            let _ = app_handle.emit("error", &format!("Backup failed: {}", e));
            return;
        }

        // Extract archive
        let _ = app_handle.emit("status-update", "Extracting update...");
        if let Err(e) = Extractor::extract_archive(&archive_path, &installation_path).await {
            let _ = app_handle.emit("error", &format!("Extraction failed: {}", e));
            // Try to restore backup
            let _ = restore_user_data(&installation_path, &config.preserve_folders).await;
            return;
        }

        // Restore user data
        let _ = app_handle.emit("status-update", "Restoring user data...");
        if let Err(e) = restore_user_data(&installation_path, &config.preserve_folders).await {
            let _ = app_handle.emit("error", &format!("Failed to restore user data: {}", e));
            return;
        }

        // Update config with new version
        let mut updated_config = config.clone();
        if let Ok(update_info) = UpdateManager::check_for_updates(&config).await {
            updated_config.current_version = update_info.latest_version;
        }
        updated_config.last_check = chrono::Utc::now().to_rfc3339();
        
        if let Err(e) = ConfigManager::save_config(&updated_config) {
            let _ = app_handle.emit("error", &format!("Failed to update config: {}", e));
            return;
        }

        // Clean up
        let _ = app_handle.emit("status-update", "Cleaning up...");
        let cache_dir = Downloader::get_cache_directory();
        let _ = Downloader::manage_cache(&cache_dir, false);
        let _ = cleanup_temp_backup().await;

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
    
    app.dialog()
        .file()
        .pick_folder(move |result| {
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

async fn backup_user_data(installation_path: &Path, preserve_folders: &[String]) -> Result<(), anyhow::Error> {
    let backup_dir = PathBuf::from("temp_backup");
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
        println!("Backed up MakePlace config.json from: {}", config_source.display());
    }

    Ok(())
}

async fn restore_user_data(installation_path: &Path, preserve_folders: &[String]) -> Result<(), anyhow::Error> {
    let backup_dir = PathBuf::from("temp_backup");

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
        println!("Restored MakePlace config.json to: {}", config_dest.display());
    }

    Ok(())
}

/// Smart config.json merging that preserves user settings while adding new options
async fn merge_config_files(backup_config: &Path, new_config: &Path) -> Result<(), anyhow::Error> {
    // Read the backed up (user) config
    let user_config_content = std::fs::read_to_string(backup_config)
        .context("Failed to read user config.json")?;
    let mut user_config: serde_json::Value = serde_json::from_str(&user_config_content)
        .context("Failed to parse user config.json")?;

    // Read the new (from update) config if it exists
    if new_config.exists() {
        let new_config_content = std::fs::read_to_string(new_config)
            .context("Failed to read new config.json")?;
        let new_config_json: serde_json::Value = serde_json::from_str(&new_config_content)
            .context("Failed to parse new config.json")?;

        // Merge: Add new keys from the update, preserve existing user values
        if let (Some(user_obj), Some(new_obj)) = (user_config.as_object_mut(), new_config_json.as_object()) {
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
    let merged_content = serde_json::to_string_pretty(&user_config)
        .context("Failed to serialize merged config")?;
    std::fs::write(new_config, merged_content)
        .context("Failed to write merged config.json")?;

    println!("Successfully merged config.json - preserved user settings and added new options");
    Ok(())
}

async fn cleanup_temp_backup() -> Result<(), anyhow::Error> {
    let backup_dir = PathBuf::from("temp_backup");
    if backup_dir.exists() {
        std::fs::remove_dir_all(&backup_dir)?;
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            validate_path,
            check_updates,
            start_download,
            install_update,
            launch_game,
            browse_folder,
            clear_cache,
            get_cache_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
