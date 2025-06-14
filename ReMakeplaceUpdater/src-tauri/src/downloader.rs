use serde::Serialize;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Write, Read};
use std::time::Instant;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Default)]
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
        Self::download_file_with_resume(url, filepath, false, progress_callback).await
    }

    pub async fn download_file_with_resume<F>(
        url: &str,
        filepath: &Path,
        resume: bool,
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

        // Check available disk space before starting download
        if let Err(e) = Self::check_disk_space(filepath).await {
            return Err(anyhow::anyhow!("Insufficient disk space: {}", e));
        }

        let client = reqwest::Client::new();
        
        // Check if we should resume download
        let mut start_byte = 0u64;
        let mut supports_range = true;
        
        if resume && filepath.exists() {
            start_byte = fs::metadata(filepath)
                .context("Failed to get file metadata for resume")?
                .len();
                
            // Only attempt resume if file has meaningful content
            if start_byte > 0 {
                // Test if server supports Range requests with a HEAD request
                match Self::test_range_support(&client, url).await {
                    Ok(true) => {
                        println!("Server supports Range requests, resuming download from byte {}", start_byte);
                    }
                    Ok(false) => {
                        println!("Server doesn't support Range requests, restarting download");
                        supports_range = false;
                        start_byte = 0;
                        // Remove the partial file since we can't resume
                        if let Err(e) = fs::remove_file(filepath) {
                            println!("Warning: Failed to remove partial file for restart: {}", e);
                        }
                    }
                    Err(e) => {
                        println!("Could not test Range support ({}), attempting resume anyway", e);
                        // Continue with resume attempt - server might still support it
                    }
                }
            }
        }

        let mut request = client.get(url);
        
        // Add Range header for resume only if server supports it
        if start_byte > 0 && supports_range {
            request = request.header("Range", format!("bytes={}-", start_byte));
        }

        let response = request
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
        }

        let total_size = if let Some(content_length) = response.content_length() {
            content_length + start_byte
        } else {
            // Try to get total size from Content-Range header for partial content
            if let Some(content_range) = response.headers().get("content-range") {
                if let Ok(range_str) = content_range.to_str() {
                    if let Some(total_str) = range_str.split('/').nth(1) {
                        total_str.parse::<u64>().unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        };

        let mut file = if start_byte > 0 {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(filepath)
                .context("Failed to open file for resume")?
        } else {
            std::fs::File::create(filepath)
                .context("Failed to create download file")?
        };

        let mut downloaded = start_byte;
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
                    ((downloaded - start_byte) as f64) / (1024.0 * 1024.0) / elapsed // MB/s
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
            ((downloaded - start_byte) as f64) / (1024.0 * 1024.0) / elapsed
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

    async fn test_range_support(client: &reqwest::Client, url: &str) -> Result<bool> {
        // Send a HEAD request to check if server accepts Range requests
        let response = client
            .head(url)
            .send()
            .await
            .context("Failed to send HEAD request for Range support test")?;

        // Check if server advertises Range support
        if let Some(accept_ranges) = response.headers().get("accept-ranges") {
            if let Ok(accept_ranges_str) = accept_ranges.to_str() {
                return Ok(accept_ranges_str.to_lowercase().contains("bytes"));
            }
        }

        // If no Accept-Ranges header, try a small range request as a test
        let test_response = client
            .get(url)
            .header("Range", "bytes=0-0")
            .send()
            .await
            .context("Failed to send test Range request")?;

        // If we get 206 Partial Content, server supports ranges
        Ok(test_response.status() == reqwest::StatusCode::PARTIAL_CONTENT)
    }

    async fn check_disk_space(filepath: &Path) -> Result<()> {
        // Get the directory where the file will be stored
        let dir = filepath.parent().unwrap_or(Path::new("."));
        
        // Try to get available space (this is platform-specific)
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem;
            use std::os::unix::ffi::OsStrExt;
            
            let path_cstr = CString::new(dir.as_os_str().as_bytes())
                .context("Failed to convert path to CString")?;
            
            let mut statvfs: libc::statvfs = unsafe { mem::zeroed() };
            let result = unsafe { libc::statvfs(path_cstr.as_ptr(), &mut statvfs) };
            
            if result == 0 {
                // Cast to u64 to handle different platforms (macOS vs Linux have different field types)
                let available_bytes = (statvfs.f_bavail as u64) * (statvfs.f_frsize as u64);
                let min_required = 100 * 1024 * 1024; // Require at least 100MB free
                
                if available_bytes < min_required {
                    return Err(anyhow::anyhow!(
                        "Not enough disk space. Available: {} MB, Required: {} MB",
                        available_bytes / (1024 * 1024),
                        min_required / (1024 * 1024)
                    ));
                }
                
                println!("Disk space check passed. Available: {} MB", available_bytes / (1024 * 1024));
            } else {
                println!("Warning: Could not check disk space, proceeding anyway");
            }
        }
        
        #[cfg(windows)]
        {
            use std::fs;
            // Basic space check for Windows - create a small test file
            let test_file = dir.join(".space_test");
            match fs::File::create(&test_file) {
                Ok(_) => {
                    let _ = fs::remove_file(&test_file);
                    println!("Basic disk write test passed");
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Cannot write to disk: {}", e));
                }
            }
        }
        
        #[cfg(not(any(unix, windows)))]
        {
            println!("Disk space check not implemented for this platform");
        }
        
        Ok(())
    }

    pub fn validate_cached_file(filepath: &Path, expected_size: Option<u64>) -> Result<bool> {
        if !filepath.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(filepath)
            .context("Failed to get file metadata")?;
        
        let file_size = metadata.len();
        
        // If we have expected size, check if file is complete
        if let Some(expected) = expected_size {
            if file_size != expected {
                println!("File size mismatch: expected {}, got {}", expected, file_size);
                return Ok(false);
            }
        }

        // Basic file validation - check if file is empty or too small
        if file_size == 0 {
            println!("File is empty");
            return Ok(false);
        }
        
        // For very small files (< 1KB), they're likely incomplete
        if file_size < 1024 {
            println!("File is suspiciously small: {} bytes", file_size);
            return Ok(false);
        }

        // Try to read first few bytes to make sure file is accessible
        match fs::File::open(filepath) {
            Ok(mut file) => {
                let mut buffer = [0u8; 16];
                match file.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        println!("File validation passed: {} bytes", file_size);
                        Ok(true)
                    }
                    _ => {
                        println!("File is not readable");
                        Ok(false)
                    }
                }
            }
            Err(_) => {
                println!("Cannot open file for validation");
                Ok(false)
            }
        }
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

 