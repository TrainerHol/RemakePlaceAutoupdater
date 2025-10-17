use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub struct Launcher;

impl Launcher {
    pub async fn launch_game(installation_path: &Path, exe_name: &str) -> Result<()> {
        let exe_path = installation_path.join(exe_name);

        if !Self::validate_executable(&exe_path)? {
            return Err(anyhow::anyhow!(
                "Executable not found: {}",
                exe_path.display()
            ));
        }

        // Launch the game as a detached process
        let mut command = Command::new(&exe_path);
        command.current_dir(installation_path);

        // On Windows, we can use CREATE_NEW_PROCESS_GROUP to detach
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
        }

        // Spawn the process without waiting for it to complete
        let child = command
            .spawn()
            .context("Failed to launch game executable")?;

        // Log the process ID for reference
        println!("Game launched with PID: {}", child.id());

        Ok(())
    }

    pub fn validate_executable(exe_path: &Path) -> Result<bool> {
        if !exe_path.exists() {
            return Ok(false);
        }

        if !exe_path.is_file() {
            return Ok(false);
        }

        // Additional validation: check if it's actually executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata =
                std::fs::metadata(exe_path).context("Failed to get executable metadata")?;
            let permissions = metadata.permissions();

            // Check if owner has execute permission
            Ok(permissions.mode() & 0o100 != 0)
        }

        #[cfg(windows)]
        {
            // On Windows, check if it's a .exe file
            Ok(exe_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase() == "exe")
                .unwrap_or(false))
        }
    }
}
