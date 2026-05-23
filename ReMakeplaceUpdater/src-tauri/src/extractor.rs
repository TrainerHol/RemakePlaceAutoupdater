use anyhow::{Context, Result};
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

type ProgressCallback = Arc<dyn Fn(String) + Send + Sync>;

pub struct Extractor;

impl Extractor {
    pub async fn extract_archive_with_progress(
        archive_path: &Path,
        destination: &Path,
        progress_callback: ProgressCallback,
    ) -> Result<()> {
        Self::extract_archive_internal(archive_path, destination, Some(progress_callback)).await
    }

    async fn extract_archive_internal(
        archive_path: &Path,
        destination: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        if !archive_path.exists() {
            return Err(anyhow::anyhow!("Archive file does not exist"));
        }

        // Create destination directory if it doesn't exist
        fs::create_dir_all(destination).context("Failed to create destination directory")?;

        // Determine extraction method based on file extension and magic bytes
        let file_name = archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        println!("Extracting archive: {}", file_name);

        let mut errors = Vec::new();

        macro_rules! try_method {
            ($label:literal, $future:expr) => {
                match $future.await {
                    Ok(()) => {
                        println!("Successfully extracted using: {}", $label);
                        return Ok(());
                    }
                    Err(e) => {
                        println!("{} failed: {}", $label, e);
                        errors.push(format!("{}: {}", $label, e));
                    }
                }
            };
        }

        try_method!(
            "7z detection",
            Self::try_extract_7z(archive_path, destination, progress_callback.clone())
        );
        try_method!(
            "ZIP detection",
            Self::try_extract_zip(archive_path, destination)
        );
        try_method!(
            "TAR.GZ detection",
            Self::try_extract_tar_gz(archive_path, destination)
        );
        try_method!(
            "TAR.BZ2 detection",
            Self::try_extract_tar_bz2(archive_path, destination)
        );
        try_method!(
            "TAR.XZ detection",
            Self::try_extract_tar_xz(archive_path, destination)
        );
        try_method!(
            "TAR.ZST detection",
            Self::try_extract_tar_zst(archive_path, destination)
        );
        try_method!(
            "TAR detection",
            Self::try_extract_tar(archive_path, destination)
        );
        try_method!(
            "GZ detection",
            Self::try_extract_gz(archive_path, destination)
        );
        try_method!(
            "BZ2 detection",
            Self::try_extract_bz2(archive_path, destination)
        );
        try_method!(
            "XZ detection",
            Self::try_extract_xz(archive_path, destination)
        );
        try_method!(
            "ZST detection",
            Self::try_extract_zst(archive_path, destination)
        );

        Err(anyhow::anyhow!(
            "No extraction method succeeded: {}",
            errors.join("; ")
        ))
    }

    async fn try_extract_7z(
        archive_path: &Path,
        destination: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        // Check if this is likely a 7z file
        let file_name = archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Only try 7z extraction for files that could be 7z archives
        if !file_name.to_lowercase().ends_with(".7z") && !Self::is_7z_file(archive_path)? {
            return Err(anyhow::anyhow!("Not a 7z file"));
        }

        println!("Attempting 7z extraction with sevenz-rust...");

        let archive_path = archive_path.to_path_buf();
        let destination = destination.to_path_buf();
        tokio::task::spawn_blocking(move || {
            Self::extract_7z_sync(&archive_path, &destination, progress_callback)
        })
        .await
        .context("7z extraction task failed")??;

        Ok(())
    }

    fn extract_7z_sync(
        archive_path: &Path,
        destination: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        if let Some(callback) = progress_callback.as_ref() {
            callback("Reading 7z archive index...".to_string());
        }

        sevenz_rust::decompress_file_with_extract_fn(
            archive_path,
            destination,
            |entry, reader, dest| {
                Self::extract_7z_entry(entry, reader, dest, progress_callback.as_ref())
            },
        )
        .context("Failed to extract 7z archive with sevenz-rust")?;

        Ok(())
    }

    fn extract_7z_entry(
        entry: &sevenz_rust::SevenZArchiveEntry,
        reader: &mut dyn Read,
        dest: &PathBuf,
        progress_callback: Option<&ProgressCallback>,
    ) -> Result<bool, sevenz_rust::Error> {
        if entry.is_directory() {
            fs::create_dir_all(dest).map_err(sevenz_rust::Error::io)?;
            return Ok(true);
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(sevenz_rust::Error::io)?;
        }

        let file = fs::File::create(dest).map_err(sevenz_rust::Error::io)?;
        let mut writer = BufWriter::new(file);
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut copied = 0u64;
        let total = entry.size();
        let display_name = Self::display_entry_name(entry.name());
        let mut last_emit = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);

        if let Some(callback) = progress_callback {
            callback(format!("Extracting {}...", display_name));
        }

        loop {
            let read = reader.read(&mut buffer).map_err(sevenz_rust::Error::io)?;
            if read == 0 {
                break;
            }

            writer
                .write_all(&buffer[..read])
                .map_err(sevenz_rust::Error::io)?;
            copied += read as u64;

            if let Some(callback) = progress_callback {
                if total > 0 && last_emit.elapsed() >= Duration::from_millis(700) {
                    let percent = ((copied as f64 / total as f64) * 100.0).clamp(0.0, 100.0);
                    callback(format!("Extracting {} ({:.0}%)", display_name, percent));
                    last_emit = Instant::now();
                }
            }
        }

        writer.flush().map_err(sevenz_rust::Error::io)?;

        Ok(true)
    }

    fn display_entry_name(name: &str) -> String {
        const MAX_LEN: usize = 64;
        if name.chars().count() <= MAX_LEN {
            return name.to_string();
        }

        let tail: String = name
            .chars()
            .rev()
            .take(MAX_LEN - 3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("...{}", tail)
    }

    async fn try_extract_zip(archive_path: &Path, destination: &Path) -> Result<()> {
        println!("Attempting ZIP extraction...");

        let file = fs::File::open(archive_path).context("Failed to open zip file")?;

        let mut archive = zip::ZipArchive::new(file).context("Failed to read zip archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("Failed to read zip entry")?;

            let outpath = match file.enclosed_name() {
                Some(path) => destination.join(path),
                None => continue, // Skip entries with invalid names
            };

            if file.is_dir() {
                fs::create_dir_all(&outpath).context("Failed to create directory")?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p).context("Failed to create parent directory")?;
                    }
                }

                let mut outfile =
                    fs::File::create(&outpath).context("Failed to create output file")?;

                std::io::copy(&mut file, &mut outfile).context("Failed to extract file")?;
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

    async fn try_extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".tar.gz", ".tgz"]) {
            return Err(anyhow::anyhow!("Not a tar.gz file"));
        }

        println!("Attempting TAR.GZ extraction...");

        let file = fs::File::open(archive_path).context("Failed to open tar.gz file")?;

        let decompressor = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);

        archive
            .unpack(destination)
            .context("Failed to extract tar.gz archive")?;

        Ok(())
    }

    async fn try_extract_tar_bz2(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".tar.bz2", ".tbz2", ".tbz"]) {
            return Err(anyhow::anyhow!("Not a tar.bz2 file"));
        }

        println!("Attempting TAR.BZ2 extraction...");

        let file = fs::File::open(archive_path).context("Failed to open tar.bz2 file")?;

        let decompressor = bzip2::read::BzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);

        archive
            .unpack(destination)
            .context("Failed to extract tar.bz2 archive")?;

        Ok(())
    }

    async fn try_extract_tar_xz(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".tar.xz", ".txz"]) {
            return Err(anyhow::anyhow!("Not a tar.xz file"));
        }

        println!("Attempting TAR.XZ extraction...");

        let file = fs::File::open(archive_path).context("Failed to open tar.xz file")?;

        let decompressor = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);

        archive
            .unpack(destination)
            .context("Failed to extract tar.xz archive")?;

        Ok(())
    }

    async fn try_extract_tar_zst(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".tar.zst", ".tar.zstd"]) {
            return Err(anyhow::anyhow!("Not a tar.zst file"));
        }

        println!("Attempting TAR.ZST extraction...");

        let file = fs::File::open(archive_path).context("Failed to open tar.zst file")?;

        let decompressor =
            zstd::stream::read::Decoder::new(file).context("Failed to create zstd decoder")?;
        let mut archive = tar::Archive::new(decompressor);

        archive
            .unpack(destination)
            .context("Failed to extract tar.zst archive")?;

        Ok(())
    }

    async fn try_extract_tar(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".tar"]) {
            return Err(anyhow::anyhow!("Not a tar file"));
        }

        println!("Attempting TAR extraction...");

        let file = fs::File::open(archive_path).context("Failed to open tar file")?;

        let mut archive = tar::Archive::new(file);

        archive
            .unpack(destination)
            .context("Failed to extract tar archive")?;

        Ok(())
    }

    async fn try_extract_gz(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".gz"])
            || Self::file_has_extensions(archive_path, &[".tar.gz", ".tgz"])
        {
            return Err(anyhow::anyhow!("Not a standalone gz file"));
        }

        println!("Attempting GZ extraction...");

        let file = fs::File::open(archive_path).context("Failed to open gz file")?;

        let mut decompressor = flate2::read::GzDecoder::new(file);

        // Extract to a file with the same name but without .gz extension
        let output_name = archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted_file");
        let output_path = destination.join(output_name);

        let mut output_file =
            fs::File::create(&output_path).context("Failed to create output file")?;

        std::io::copy(&mut decompressor, &mut output_file)
            .context("Failed to decompress gz file")?;

        Ok(())
    }

    async fn try_extract_bz2(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".bz2"])
            || Self::file_has_extensions(archive_path, &[".tar.bz2", ".tbz2", ".tbz"])
        {
            return Err(anyhow::anyhow!("Not a standalone bz2 file"));
        }

        println!("Attempting BZ2 extraction...");

        let file = fs::File::open(archive_path).context("Failed to open bz2 file")?;

        let mut decompressor = bzip2::read::BzDecoder::new(file);

        // Extract to a file with the same name but without .bz2 extension
        let output_name = archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted_file");
        let output_path = destination.join(output_name);

        let mut output_file =
            fs::File::create(&output_path).context("Failed to create output file")?;

        std::io::copy(&mut decompressor, &mut output_file)
            .context("Failed to decompress bz2 file")?;

        Ok(())
    }

    async fn try_extract_xz(archive_path: &Path, destination: &Path) -> Result<()> {
        if !Self::file_has_extensions(archive_path, &[".xz"])
            || Self::file_has_extensions(archive_path, &[".tar.xz", ".txz"])
        {
            return Err(anyhow::anyhow!("Not a standalone xz file"));
        }

        println!("Attempting XZ extraction...");

        let file = fs::File::open(archive_path).context("Failed to open xz file")?;

        let mut decompressor = xz2::read::XzDecoder::new(file);

        // Extract to a file with the same name but without .xz extension
        let output_name = archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted_file");
        let output_path = destination.join(output_name);

        let mut output_file =
            fs::File::create(&output_path).context("Failed to create output file")?;

        std::io::copy(&mut decompressor, &mut output_file)
            .context("Failed to decompress xz file")?;

        Ok(())
    }

    async fn try_extract_zst(archive_path: &Path, destination: &Path) -> Result<()> {
        // More permissive check - try zst extraction if file has zst extension
        if !Self::file_has_extensions(archive_path, &[".zst", ".zstd"]) {
            return Err(anyhow::anyhow!("Not a zst file"));
        }

        println!("Attempting ZST extraction...");

        let file = fs::File::open(archive_path).context("Failed to open zst file")?;

        // Try to create decoder first to validate it's a valid zst file
        let mut decompressor = zstd::stream::read::Decoder::new(file)
            .context("Failed to create zstd decoder - file may not be a valid ZST archive")?;

        // For .tar.zst files, we should have caught them earlier, but if we get here,
        // it might be a misnamed standalone zst file, so try to extract it anyway
        let output_name = if Self::file_has_extensions(archive_path, &[".tar.zst", ".tar.zstd"]) {
            // If it has tar.zst extension but we're treating it as standalone zst,
            // extract with .tar extension so it can be processed by tar extraction later
            archive_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("extracted_file")
        } else {
            // Normal standalone zst file - remove .zst extension
            archive_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("extracted_file")
        };

        let output_path = destination.join(output_name);

        let mut output_file =
            fs::File::create(&output_path).context("Failed to create output file")?;

        std::io::copy(&mut decompressor, &mut output_file).context(
            "Failed to decompress zst file - the file may be corrupted or not a valid ZST archive",
        )?;

        println!(
            "Successfully extracted ZST file to: {}",
            output_path.display()
        );
        Ok(())
    }

    fn file_has_extensions(path: &Path, extensions: &[&str]) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        extensions
            .iter()
            .any(|ext| file_name.to_lowercase().ends_with(&ext.to_lowercase()))
    }

    fn is_7z_file(path: &Path) -> Result<bool> {
        // Check for 7z magic bytes at the beginning of the file
        let mut file = fs::File::open(path).context("Failed to open file for magic byte check")?;

        let mut magic = [0u8; 6];
        match file.read_exact(&mut magic) {
            Ok(()) => {
                // 7z files start with "7z¼¯'" (0x377ABCAF271C)
                Ok(magic == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C])
            }
            Err(_) => Ok(false), // File too small or read error
        }
    }
}
