use std::path::Path;
use std::fs;
use std::process::Command;
use anyhow::{Result, Context};

pub struct Extractor;

impl Extractor {
    pub async fn extract_archive(
        archive_path: &Path,
        destination: &Path,
    ) -> Result<()> {
        if !archive_path.exists() {
            return Err(anyhow::anyhow!("Archive file does not exist"));
        }

        // Create destination directory if it doesn't exist
        fs::create_dir_all(destination)
            .context("Failed to create destination directory")?;

        // Determine extraction method based on file extension
        let file_name = archive_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name.ends_with(".7z") {
            Self::extract_7z_with_fallback(archive_path, destination).await
        } else if file_name.ends_with(".zip") {
            Self::extract_zip(archive_path, destination).await
        } else {
            // Try 7z first, then zip as fallback
            match Self::extract_7z_with_fallback(archive_path, destination).await {
                Ok(()) => Ok(()),
                Err(_) => Self::extract_zip(archive_path, destination).await,
            }
        }
    }

    async fn extract_7z_with_fallback(
        archive_path: &Path,
        destination: &Path,
    ) -> Result<()> {
        // Try native 7z extraction first
        match Self::extract_7z_native(archive_path, destination).await {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("Native 7z extraction failed: {}, trying system 7zip...", e);
                Self::extract_with_system_7z(archive_path, destination).await
            }
        }
    }

    async fn extract_7z_native(
        archive_path: &Path,
        destination: &Path,
    ) -> Result<()> {
        // Use the sevenz-rust crate properly
        sevenz_rust::decompress_file(archive_path, destination)
            .context("Failed to extract 7z archive with native extractor")?;

        Ok(())
    }

    async fn extract_with_system_7z(
        archive_path: &Path,
        destination: &Path,
    ) -> Result<()> {
        // Try common 7zip installation paths
        let seven_zip_paths = [
            "7z.exe",
            "C:\\Program Files\\7-Zip\\7z.exe",
            "C:\\Program Files (x86)\\7-Zip\\7z.exe",
        ];

        let mut last_error = None;

        for zip_path in &seven_zip_paths {
            match Command::new(zip_path)
                .args(&[
                    "x",
                    archive_path.to_str().unwrap(),
                    &format!("-o{}", destination.to_str().unwrap()),
                    "-y", // Assume yes for all queries
                ])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        return Ok(());
                    } else {
                        last_error = Some(anyhow::anyhow!(
                            "7zip extraction failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("Failed to run 7zip: {}", e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No 7zip executable found")))
    }

    async fn extract_zip(
        archive_path: &Path,
        destination: &Path,
    ) -> Result<()> {
        let file = std::fs::File::open(archive_path)
            .context("Failed to open zip file")?;

        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read zip archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .context("Failed to read zip entry")?;

            let outpath = match file.enclosed_name() {
                Some(path) => destination.join(path),
                None => continue, // Skip entries with invalid names
            };

            if file.name().ends_with('/') {
                // Directory
                fs::create_dir_all(&outpath)
                    .context("Failed to create directory")?;
            } else {
                // File
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)
                            .context("Failed to create parent directory")?;
                    }
                }

                let mut outfile = std::fs::File::create(&outpath)
                    .context("Failed to create output file")?;

                std::io::copy(&mut file, &mut outfile)
                    .context("Failed to extract file")?;
            }

            // Set file permissions on Unix-like systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                        .context("Failed to set file permissions")?;
                }
            }
        }

        Ok(())
    }


} 