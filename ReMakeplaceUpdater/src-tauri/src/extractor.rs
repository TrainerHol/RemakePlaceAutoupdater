use anyhow::{Context, Result};
use std::fs;
use std::io::Read;
use std::path::Path;

pub struct Extractor;

impl Extractor {
    pub async fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
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

        // Try extraction methods in order of preference
        let mut last_error = None;

        // Try 7z extraction
        match Self::try_extract_7z(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: 7z detection");
                return Ok(());
            }
            Err(e) => {
                println!("7z detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try ZIP extraction
        match Self::try_extract_zip(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: ZIP detection");
                return Ok(());
            }
            Err(e) => {
                println!("ZIP detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try TAR.GZ extraction
        match Self::try_extract_tar_gz(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: TAR.GZ detection");
                return Ok(());
            }
            Err(e) => {
                println!("TAR.GZ detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try TAR.BZ2 extraction
        match Self::try_extract_tar_bz2(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: TAR.BZ2 detection");
                return Ok(());
            }
            Err(e) => {
                println!("TAR.BZ2 detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try TAR.XZ extraction
        match Self::try_extract_tar_xz(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: TAR.XZ detection");
                return Ok(());
            }
            Err(e) => {
                println!("TAR.XZ detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try TAR.ZST extraction
        match Self::try_extract_tar_zst(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: TAR.ZST detection");
                return Ok(());
            }
            Err(e) => {
                println!("TAR.ZST detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try TAR extraction
        match Self::try_extract_tar(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: TAR detection");
                return Ok(());
            }
            Err(e) => {
                println!("TAR detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try GZ extraction
        match Self::try_extract_gz(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: GZ detection");
                return Ok(());
            }
            Err(e) => {
                println!("GZ detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try BZ2 extraction
        match Self::try_extract_bz2(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: BZ2 detection");
                return Ok(());
            }
            Err(e) => {
                println!("BZ2 detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try XZ extraction
        match Self::try_extract_xz(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: XZ detection");
                return Ok(());
            }
            Err(e) => {
                println!("XZ detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // Try ZST extraction
        match Self::try_extract_zst(archive_path, destination).await {
            Ok(()) => {
                println!("Successfully extracted using: ZST detection");
                return Ok(());
            }
            Err(e) => {
                println!("ZST detection failed: {}", e);
                last_error = Some(e);
            }
        }

        // If we reach here, no extraction method succeeded
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No extraction method succeeded")))
    }

    async fn try_extract_7z(archive_path: &Path, destination: &Path) -> Result<()> {
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

        // Try with sevenz-rust
        sevenz_rust::decompress_file(archive_path, destination)
            .context("Failed to extract 7z archive with sevenz-rust")?;

        Ok(())
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
