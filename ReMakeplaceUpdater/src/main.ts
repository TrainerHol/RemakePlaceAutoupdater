import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, UpdateInfo, ProgressInfo, AppStatus } from "./types";
import { AppState } from "./types";

class ReMakeplaceUpdater {
  private config: Config | null = null;
  private updateInfo: UpdateInfo | null = null;
  private currentStatus: AppStatus = {
    state: AppState.IDLE,
    message: "Initializing...",
  };
  private isFirstRun = false;

  // UI Elements
  private statusMessage!: HTMLElement;
  private currentVersionElement!: HTMLElement;
  private latestVersionElement!: HTMLElement;
  private installationPathElement!: HTMLElement;
  private progressBar!: HTMLElement;
  private progressText!: HTMLElement;
  private updateButton!: HTMLButtonElement;
  private launchButton!: HTMLButtonElement;
  private settingsButton!: HTMLButtonElement;
  private clearCacheButton!: HTMLButtonElement;
  private progressSection!: HTMLElement;

  constructor() {
    this.initializeUI();
    this.setupEventListeners();
    this.loadConfiguration();
  }

  private initializeUI() {
    document.body.innerHTML = `
      <div class="app-container">
        <!-- Header Section -->
        <div class="header">
          <h1>ReMakeplace Launcher</h1>
        </div>

        <!-- Content Wrapper -->
        <div class="content-wrapper">
          <!-- Installation Path Section -->
          <div class="section">
            <div class="path-display">
              <span class="path-label">Installation Path:</span>
              <span id="installation-path" class="path-text">Not configured</span>
              <button id="settings-btn" class="settings-btn">⚙️ Settings</button>
            </div>
          </div>

          <!-- Version Information Section -->
          <div class="section">
            <div class="version-info">
              <div class="version-item">
                <span class="version-label">Current Version:</span>
                <span id="current-version" class="version-text">Unknown</span>
              </div>
              <div class="version-item">
                <span class="version-label">Latest Version:</span>
                <span id="latest-version" class="version-text">Checking...</span>
              </div>
            </div>
            <div id="status-message" class="status-message">Initializing...</div>
          </div>

          <!-- Progress Section (hidden by default) -->
          <div id="progress-section" class="section progress-section" style="display: none;">
            <div class="progress-container">
              <div id="progress-bar" class="progress-bar">
                <div class="progress-fill"></div>
              </div>
              <div id="progress-text" class="progress-text">0% - 0.0 MB/s</div>
            </div>
          </div>

          <!-- Button Section -->
          <div class="section button-section">
            <button id="update-btn" class="btn btn-primary" disabled>Check for Updates</button>
            <button id="launch-btn" class="btn btn-secondary">Launch ReMakeplace</button>
            <button id="clear-cache-btn" class="btn btn-small">Clear Cache</button>
          </div>
        </div>

        <!-- Settings Modal (hidden by default) -->
        <div id="settings-modal" class="modal" style="display: none;">
          <div class="modal-content">
            <div class="modal-header">
              <h2>Settings</h2>
            </div>
            <div class="modal-body">
              <div class="form-group">
                <label for="path-input">Installation Path:</label>
                <div class="path-input-group">
                  <input type="text" id="path-input" class="path-input" placeholder="Select installation folder...">
                  <button id="browse-btn" class="btn btn-small">Browse</button>
                </div>
                <div id="path-validation" class="validation-message"></div>
              </div>
            </div>
            <div class="modal-footer">
              <button id="cancel-btn" class="btn btn-secondary">Cancel</button>
              <button id="save-btn" class="btn btn-primary" disabled>Save & Continue</button>
            </div>
          </div>
        </div>
      </div>
    `;

    // Get references to UI elements
    this.statusMessage = document.getElementById("status-message")!;
    this.currentVersionElement = document.getElementById("current-version")!;
    this.latestVersionElement = document.getElementById("latest-version")!;
    this.installationPathElement = document.getElementById("installation-path")!;
    this.progressBar = document.getElementById("progress-bar")!;
    this.progressText = document.getElementById("progress-text")!;
    this.updateButton = document.getElementById("update-btn") as HTMLButtonElement;
    this.launchButton = document.getElementById("launch-btn") as HTMLButtonElement;
    this.settingsButton = document.getElementById("settings-btn") as HTMLButtonElement;
    this.clearCacheButton = document.getElementById("clear-cache-btn") as HTMLButtonElement;
    this.progressSection = document.getElementById("progress-section")!;
  }

  private setupEventListeners() {
    // Tauri event listeners
    listen<ProgressInfo>("download-progress", (event) => {
      this.updateProgress(event.payload);
    });

    listen<string>("download-complete", () => {
      this.onDownloadComplete();
    });

    listen<string>("download-error", (event) => {
      const errorMsg = event.payload;
      let userFriendlyMsg = "Download failed";
      
      if (errorMsg.includes("network") || errorMsg.includes("connection")) {
        userFriendlyMsg = "Download failed due to network issues. Check your internet connection and try again.";
      } else if (errorMsg.includes("timeout")) {
        userFriendlyMsg = "Download timed out. Try clearing cache and retrying.";
      } else if (errorMsg.includes("validation")) {
        userFriendlyMsg = "Downloaded file is corrupted. Try clearing cache and downloading again.";
      } else if (errorMsg.includes("space") || errorMsg.includes("disk")) {
        userFriendlyMsg = "Not enough disk space. Free up some space and try again.";
      }
      
      this.setStatus(AppState.ERROR, `${userFriendlyMsg} (${errorMsg})`);
    });

    listen<string>("status-update", (event) => {
      this.setStatus(AppState.INSTALLING, event.payload);
    });

    listen<string>("error", (event) => {
      const errorMsg = event.payload;
      let userFriendlyMsg = "An error occurred";
      
      if (errorMsg.includes("Extraction failed")) {
        if (errorMsg.includes("zst")) {
          userFriendlyMsg = "Archive extraction failed. The downloaded file may be corrupted or in an unsupported format. Try clearing cache and downloading again.";
        } else {
          userFriendlyMsg = "Failed to extract the update archive. The file may be corrupted.";
        }
      } else if (errorMsg.includes("Backup failed")) {
        userFriendlyMsg = "Failed to backup your data before updating. Check that you have sufficient disk space.";
      } else if (errorMsg.includes("Failed to restore")) {
        userFriendlyMsg = "Update completed but failed to restore some user data. Check your installation directory.";
      }
      
      this.setStatus(AppState.ERROR, `${userFriendlyMsg} (${errorMsg})`);
    });

    listen("update-complete", () => {
      this.onUpdateComplete();
    });

    // UI event listeners
    this.updateButton.addEventListener("click", () => {
      if (this.currentStatus.state === AppState.UPDATE_AVAILABLE) {
        this.startUpdate();
      } else {
        this.checkForUpdates();
      }
    });

    this.launchButton.addEventListener("click", () => {
      this.launchGame();
    });

    this.settingsButton.addEventListener("click", () => {
      this.showSettings();
    });

    this.clearCacheButton.addEventListener("click", () => {
      this.clearCache();
    });

    // Settings modal listeners
    const modal = document.getElementById("settings-modal")!;
    const pathInput = document.getElementById("path-input") as HTMLInputElement;
    const browseBtn = document.getElementById("browse-btn")!;
    const cancelBtn = document.getElementById("cancel-btn")!;
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;

    pathInput.addEventListener("input", () => {
      this.validatePath(pathInput.value);
    });

    browseBtn.addEventListener("click", () => {
      this.browseFolder();
    });

    cancelBtn.addEventListener("click", () => {
      modal.style.display = "none";
    });

    saveBtn.addEventListener("click", () => {
      this.savePath(pathInput.value);
    });

    // Close modal when clicking outside
    modal.addEventListener("click", (e) => {
      if (e.target === modal) {
        modal.style.display = "none";
      }
    });
  }

  private async loadConfiguration() {
    try {
      this.config = await invoke<Config>("load_config");
      this.updateUI();

      if (!this.config.installation_path || !(await this.validateInstallationPath())) {
        this.isFirstRun = true;
        this.showSettings(true);
      } else {
        this.checkForUpdates();
      }
    } catch (error) {
      console.error("Failed to load configuration:", error);
      this.setStatus(AppState.ERROR, "Failed to load configuration");
    }
  }

  private async validateInstallationPath(): Promise<boolean> {
    if (!this.config) return false;

    try {
      return await invoke<boolean>("validate_path", {
        path: this.config.installation_path,
        exeName: this.config.exe_path,
      });
    } catch (error) {
      return false;
    }
  }

  private updateUI() {
    if (!this.config) return;

    this.currentVersionElement.textContent = this.config.current_version;
    this.installationPathElement.textContent = this.config.installation_path || "Not configured";

    // Update launch button state
    this.launchButton.disabled = !this.config.installation_path;
  }

  private async checkForUpdates() {
    if (!this.config) return;

    this.setStatus(AppState.CHECKING_UPDATES, "Checking for updates...");
    this.updateButton.disabled = true;

    try {
      this.updateInfo = await invoke<UpdateInfo>("check_updates", { config: this.config });

      if (this.updateInfo.is_available) {
        this.latestVersionElement.textContent = this.updateInfo.latest_version;
        this.setStatus(AppState.UPDATE_AVAILABLE, `Update available: ${this.updateInfo.latest_version}`);
        this.updateButton.textContent = "Update Now";
        this.updateButton.disabled = false;
        this.updateButton.classList.add("btn-update");
      } else {
        this.latestVersionElement.textContent = this.config.current_version;
        this.setStatus(AppState.UP_TO_DATE, "You have the latest version");
        this.updateButton.textContent = "Up to Date";
        this.updateButton.disabled = true;
        this.updateButton.classList.remove("btn-update");
      }
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to check updates: ${error}`);
      this.updateButton.disabled = false;
      this.updateButton.textContent = "Retry";
    }
  }

  private async startUpdate() {
    if (!this.config || !this.updateInfo?.is_available) return;

    this.setStatus(AppState.DOWNLOADING, "Starting download...");
    this.progressSection.style.display = "block";
    this.updateButton.disabled = true;

    try {
      const filename = this.updateInfo.download_url.split("/").pop() || "update.7z";

      await invoke("start_download", {
        url: this.updateInfo.download_url,
        version: this.updateInfo.latest_version,
        originalFilename: filename,
      });
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to start download: ${error}`);
      this.progressSection.style.display = "none";
    }
  }

  private updateProgress(progress: ProgressInfo) {
    const progressFill = this.progressBar.querySelector(".progress-fill") as HTMLElement;

    progressFill.style.width = `${progress.percentage}%`;

    const speedText = progress.speed > 0 ? `${progress.speed.toFixed(1)} MB/s` : "0.0 MB/s";
    this.progressText.textContent = `${progress.percentage.toFixed(1)}% - ${speedText}`;

    this.setStatus(AppState.DOWNLOADING, `Downloading update... ${progress.percentage.toFixed(1)}%`);
  }

  private async onDownloadComplete() {
    if (!this.config || !this.updateInfo) return;

    this.setStatus(AppState.INSTALLING, "Download complete, starting installation...");

    try {
      const filename = this.updateInfo.download_url.split("/").pop() || "update.7z";
      
      // Get the cache path from the backend to ensure consistency
      const cachePath = await invoke<string>("get_cache_path", {
        version: this.updateInfo.latest_version,
        originalFilename: filename,
      });

      await invoke("install_update", {
        archivePath: cachePath,
        config: this.config,
      });
    } catch (error) {
      this.setStatus(AppState.ERROR, `Installation failed: ${error}`);
      this.progressSection.style.display = "none";
    }
  }

  private async onUpdateComplete() {
    this.setStatus(AppState.UP_TO_DATE, "Update completed successfully!");
    this.progressSection.style.display = "none";

    // Reload configuration to get updated version
    await this.loadConfiguration();

    this.updateButton.textContent = "Up to Date";
    this.updateButton.disabled = true;
    this.updateButton.classList.remove("btn-update");
  }

  private async launchGame() {
    if (!this.config) {
      this.showSettings();
      return;
    }

    try {
      await invoke("launch_game", { config: this.config });
      this.setStatus(AppState.IDLE, "Game launched successfully");
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to launch game: ${error}`);
    }
  }

  private showSettings(isFirstRun = false) {
    const modal = document.getElementById("settings-modal")!;
    const pathInput = document.getElementById("path-input") as HTMLInputElement;
    const modalHeader = modal.querySelector(".modal-header h2") as HTMLElement;

    if (isFirstRun) {
      modalHeader.textContent = "Welcome to ReMakeplace Launcher";
      const modalBody = modal.querySelector(".modal-body")!;
      modalBody.insertAdjacentHTML("afterbegin", '<p class="welcome-message">Please select your ReMakeplace installation folder to continue.</p>');
    }

    pathInput.value = this.config?.installation_path || "";
    modal.style.display = "flex";

    if (this.config?.installation_path) {
      this.validatePath(this.config.installation_path);
    }
  }

  private async validatePath(path: string) {
    const validation = document.getElementById("path-validation")!;
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;

    if (!path.trim()) {
      validation.textContent = "";
      validation.className = "validation-message";
      saveBtn.disabled = true;
      return;
    }

    try {
      const isValid = await invoke<boolean>("validate_path", {
        path: path,
        exeName: this.config?.exe_path || "Makeplace.exe",
      });

      if (isValid) {
        validation.textContent = "✅ Valid installation path";
        validation.className = "validation-message valid";
        saveBtn.disabled = false;
      } else {
        validation.textContent = "❌ Invalid path or Makeplace.exe not found";
        validation.className = "validation-message invalid";
        saveBtn.disabled = true;
      }
    } catch (error) {
      validation.textContent = "❌ Error validating path";
      validation.className = "validation-message invalid";
      saveBtn.disabled = true;
    }
  }

  private async browseFolder() {
    try {
      const selected = await invoke<string | null>("browse_folder");
      if (selected) {
        const pathInput = document.getElementById("path-input") as HTMLInputElement;
        pathInput.value = selected;
        this.validatePath(selected);
      }
    } catch (error) {
      console.error("Failed to browse folder:", error);
    }
  }

  private async savePath(path: string) {
    if (!this.config) return;

    try {
      this.config.installation_path = path;
      await invoke("save_config", { config: this.config });

      const modal = document.getElementById("settings-modal")!;
      modal.style.display = "none";

      this.updateUI();

      if (this.isFirstRun) {
        this.isFirstRun = false;
        this.checkForUpdates();
      }
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to save configuration: ${error}`);
    }
  }

  private async clearCache() {
    try {
      this.clearCacheButton.disabled = true;
      this.setStatus(AppState.IDLE, "Clearing cache...");
      
      await invoke("clear_cache");
      
      this.setStatus(AppState.IDLE, "Cache cleared successfully");
      this.clearCacheButton.disabled = false;
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to clear cache: ${error}`);
      this.clearCacheButton.disabled = false;
    }
  }

  private setStatus(state: AppState, message: string, error?: string) {
    this.currentStatus = { state, message, error };
    this.statusMessage.textContent = message;

    // Update status message styling based on state
    this.statusMessage.className = `status-message ${state}`;

    if (state === AppState.ERROR) {
      this.statusMessage.classList.add("error");
    }
  }
}

// Initialize the application when DOM is loaded
document.addEventListener("DOMContentLoaded", () => {
  new ReMakeplaceUpdater();
});
