use serde::Serialize;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use std::time::Instant;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize)]
pub struct ProgressInfo {
    pub percentage: f64,
    pub speed: f64,         // MB/s
    pub downloaded: u64,
    pub total: u64,
}

pub struct Downloader;

impl Downloader {
    pub async fn download_file<F>(
        url: &str,
        filepath: &Path,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(ProgressInfo) + Send + 'static,
    {
        // Create parent directory if it doesn't exist
        if let Some(parent) = filepath.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create cache directory")?;
        }

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = std::fs::File::create(filepath)
            .context("Failed to create download file")?;

        let mut downloaded = 0u64;
        let start_time = Instant::now();
        let mut last_update = Instant::now();

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read download chunk")?;
            file.write_all(&chunk)
                .context("Failed to write download chunk")?;

            downloaded += chunk.len() as u64;

            // Update progress every 100ms
            if last_update.elapsed().as_millis() >= 100 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    (downloaded as f64) / (1024.0 * 1024.0) / elapsed // MB/s
                } else {
                    0.0
                };

                let percentage = if total_size > 0 {
                    (downloaded as f64 / total_size as f64) * 100.0
                } else {
                    0.0
                };

                progress_callback(ProgressInfo {
                    percentage,
                    speed,
                    downloaded,
                    total: total_size,
                });

                last_update = Instant::now();
            }
        }

        // Final progress update
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            (downloaded as f64) / (1024.0 * 1024.0) / elapsed
        } else {
            0.0
        };

        progress_callback(ProgressInfo {
            percentage: 100.0,
            speed,
            downloaded,
            total: total_size,
        });

        Ok(())
    }



    pub fn manage_cache(cache_dir: &Path, keep_current: bool) -> Result<()> {
        if !cache_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(cache_dir)
            .context("Failed to read cache directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read cache entry")?;
            let path = entry.path();

            if path.is_file() {
                // Remove old cache files, keep current if specified
                if !keep_current || !Self::is_current_version_file(&path) {
                    let _ = fs::remove_file(&path); // Ignore errors for cleanup
                }
            }
        }

        Ok(())
    }

    pub fn get_cache_filepath(cache_dir: &Path, version: &str, original_filename: &str) -> PathBuf {
        let cache_filename = format!("v{}_{}", version, original_filename);
        cache_dir.join(cache_filename)
    }

    pub fn get_cache_directory() -> PathBuf {
        PathBuf::from("update_cache")
    }

    fn is_current_version_file(path: &Path) -> bool {
        // Simple heuristic: assume files starting with the latest version are current
        // This could be improved with actual version tracking
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // This is a simplified check - in a real implementation, you'd track the current version
            !filename.starts_with("v0.") // Placeholder logic
        } else {
            false
        }
    }
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self {
            percentage: 0.0,
            speed: 0.0,
            downloaded: 0,
            total: 0,
        }
    }
} 