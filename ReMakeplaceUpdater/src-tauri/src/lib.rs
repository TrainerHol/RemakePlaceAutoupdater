// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Emitter};

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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_config: Arc::new(Mutex::new(None)),
            download_progress: Arc::new(Mutex::new(ProgressInfo::default())),
            is_updating: Arc::new(Mutex::new(false)),
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
) -> Result<String, String> {
    let cache_dir = Downloader::get_cache_directory();
    let filepath = Downloader::get_cache_filepath(&cache_dir, &version, &original_filename);

    // Check if file already exists in cache
    if filepath.exists() {
        return Ok(filepath.to_string_lossy().to_string());
    }

    let app_handle_progress = app_handle.clone();
    let app_handle_complete = app_handle.clone();
    let app_handle_error = app_handle;
    let filepath_clone = filepath.clone();

    tokio::spawn(async move {
        let progress_callback = move |progress: ProgressInfo| {
            let _ = app_handle_progress.emit("download-progress", &progress);
        };

        match Downloader::download_file(&url, &filepath_clone, progress_callback).await {
            Ok(()) => {
                let _ = app_handle_complete.emit("download-complete", &filepath_clone.to_string_lossy().to_string());
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

    // Restore config.json if it was backed up
    let config_source = backup_dir.join("config.json");
    let config_dest = installation_path.join("config.json");
    if config_source.exists() {
        std::fs::copy(&config_source, &config_dest)?;
    }

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
            browse_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
