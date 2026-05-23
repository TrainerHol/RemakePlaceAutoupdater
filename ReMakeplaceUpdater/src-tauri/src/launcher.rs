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

        #[cfg(target_os = "linux")]
        if !Self::wine_available() {
            return Err(anyhow::anyhow!(
                "Wine is required to launch ReMakeplace on Linux. Install Wine, then try launching again. Installation path: {}",
                installation_path.display()
            ));
        }

        let (program, args) = Self::command_parts_for_platform(std::env::consts::OS, &exe_path);

        // Launch the game as a detached process
        let mut command = Command::new(program);
        command.args(args);
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
            #[cfg(target_os = "linux")]
            {
                return Ok(true);
            }

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

    pub fn command_parts_for_platform(platform: &str, exe_path: &Path) -> (String, Vec<String>) {
        if platform == "linux" {
            return (
                "wine".to_string(),
                vec![exe_path.to_string_lossy().to_string()],
            );
        }

        (exe_path.to_string_lossy().to_string(), Vec::new())
    }

    #[cfg(target_os = "linux")]
    fn wine_available() -> bool {
        Command::new("wine")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn non_linux_launches_executable_directly() {
        let exe = PathBuf::from("/tmp/Makeplace.exe");
        let (program, args) = Launcher::command_parts_for_platform("windows", &exe);
        assert_eq!(program, "/tmp/Makeplace.exe");
        assert!(args.is_empty());
    }

    #[test]
    fn linux_launches_through_wine_when_available() {
        let exe = PathBuf::from("/tmp/Makeplace.exe");
        let (program, args) = Launcher::command_parts_for_platform("linux", &exe);
        assert_eq!(program, "wine");
        assert_eq!(args, vec!["/tmp/Makeplace.exe".to_string()]);
    }
}
