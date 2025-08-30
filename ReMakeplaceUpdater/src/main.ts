import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, UpdateInfo, ProgressInfo, AppStatus, InstallationMode, ErrorInfo, Metadata } from "./types";
import { AppState, ErrorCategory } from "./types";

class ReMakeplaceUpdater {
  private config: Config | null = null;
  private updateInfo: UpdateInfo | null = null;
  private currentStatus: AppStatus = {
    state: AppState.IDLE,
    message: "Initializing...",
  };
  private isFirstRun = false;
  private metadata: Metadata | null = null;

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
    this.loadMetadata();
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
            <button id="clear-cache-btn" class="btn btn-small" title="Cleans up leftover downloaded update files.">Clear Cache</button>
          </div>
        </div>

        <!-- Footer Section -->
        <div class="footer">
          <div class="footer-left" id="readme-link" title="Please read the README before asking questions.">
            <span class="icon book" aria-hidden="true">
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 256 256"><path d="M210.78,39.25l-130.25-23A16,16,0,0,0,62,29.23l-29.75,169a16,16,0,0,0,13,18.53l130.25,23h0a16,16,0,0,0,18.54-13l29.75-169A16,16,0,0,0,210.78,39.25ZM178.26,224h0L48,201,77.75,32,208,55ZM89.34,58.42a8,8,0,0,1,9.27-6.48l83,14.65a8,8,0,0,1-1.39,15.88,8.36,8.36,0,0,1-1.4-.12l-83-14.66A8,8,0,0,1,89.34,58.42ZM83.8,89.94a8,8,0,0,1,9.27-6.49l83,14.66A8,8,0,0,1,174.67,114a7.55,7.55,0,0,1-1.41-.13l-83-14.65A8,8,0,0,1,83.8,89.94Zm-5.55,31.51A8,8,0,0,1,87.52,115L129,122.29a8,8,0,0,1-1.38,15.88,8.27,8.27,0,0,1-1.4-.12l-41.5-7.33A8,8,0,0,1,78.25,121.45Z"></path></svg>
            </span>
            <span class="footer-text">Please read the README before asking questions.</span>
          </div>
          <div class="footer-center" id="discord-link" title="Join the Discord">
            <span class="icon discord" aria-hidden="true">
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 256 256"><path d="M104,140a12,12,0,1,1-12-12A12,12,0,0,1,104,140Zm60-12a12,12,0,1,0,12,12A12,12,0,0,0,164,128Zm74.45,64.9-67,29.71a16.17,16.17,0,0,1-21.71-9.1l-8.11-22q-6.72.45-13.63.46t-13.63-.46l-8.11,22a16.18,16.18,0,0,1-21.71,9.1l-67-29.71a15.93,15.93,0,0,1-9.06-18.51L38,58A16.07,16.07,0,0,1,51,46.14l36.06-5.93a16.22,16.22,0,0,1,18.26,11.88l3.26,12.84Q118.11,64,128,64t19.4.93l3.26-12.84a16.21,16.21,0,0,1,18.26-11.88L205,46.14A16.07,16.07,0,0,1,218,58l29.53,116.38A15.93,15.93,0,0,1,238.45,192.9ZM232,178.28,202.47,62s0,0-.08,0L166.33,56a.17.17,0,0,0-.17,0l-2.83,11.14c5,.94,10,2.06,14.83,3.42A8,8,0,0,1,176,86.31a8.09,8.09,0,0,1-2.16-.3A172.25,172.25,0,0,0,128,80a172.25,172.25,0,0,0-45.84,6,8,8,0,1,1-4.32-15.4c4.82-1.36,9.78-2.48,14.82-3.42L89.83,56s0,0-.12,0h0L53.61,61.93a.17.17,0,0,0-.09,0L24,178.33,91,208a.23.23,0,0,0,.22,0L98,189.72a173.2,173.2,0,0,1-20.14-4.32A8,8,0,0,1,82.16,170,171.85,171.85,0,0,0,128,176a171.85,171.85,0,0,0,45.84-6,8,8,0,0,1,4.32,15.41A173.2,173.2,0,0,1,158,189.72L164.75,208a.22.22,0,0,0,.21,0Z"></path></svg>
            </span>
            <span class="footer-text">Join the Discord</span>
          </div>
        </div>

        <!-- MOTD Line (below footer) -->
        <div id="motd-line" class="motd-line" style="display: none;"></div>

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
              <div class="form-group" id="version-override-group" style="display: none;">
                <label>
                  <input type="checkbox" id="version-override" />
                  Set current version to latest
                </label>
                <div class="help-text" title="If your installation shows version 0.0.0 but is actually up to date, check this to sync with the latest version without reinstalling.">ⓘ For existing installations showing incorrect version</div>
              </div>
              <div class="form-group">
                <button id="open-config-btn" class="btn btn-small" title="Open the folder where config.json is stored">Open config folder</button>
              </div>
            </div>
            <div class="modal-footer">
              <button id="cancel-btn" class="btn btn-secondary">Cancel</button>
              <button id="save-btn" class="btn btn-primary" disabled>Save & Continue</button>
            </div>
          </div>
        </div>

        <!-- Confirmation Modal (hidden by default) -->
        <div id="confirmation-modal" class="modal" style="display: none;">
          <div class="modal-content confirmation-modal">
            <div class="modal-header">
              <h2 id="confirmation-title">Confirm Action</h2>
            </div>
            <div class="modal-body">
              <div id="confirmation-message" class="confirmation-message"></div>
            </div>
            <div class="modal-footer">
              <button id="confirmation-cancel" class="btn btn-secondary">Cancel</button>
              <button id="confirmation-confirm" class="btn btn-primary">Confirm</button>
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
    // Footer and settings helpers
    const readmeLink = document.getElementById("readme-link") as HTMLElement | null;
    const discordLink = document.getElementById("discord-link") as HTMLElement | null;
    const openConfigBtn = document.getElementById("open-config-btn") as HTMLButtonElement | null;

    if (readmeLink) {
      readmeLink.addEventListener("click", async () => {
        try {
          await invoke("open_url", { url: "https://github.com/TrainerHol/RemakePlaceAutoupdater#remakeplace-auto-updater-" });
        } catch (e) {
          console.error("Failed to open README:", e);
        }
      });
    }

    if (discordLink) {
      discordLink.addEventListener("click", async () => {
        try {
          const url = this.getDiscordInvite();
          await invoke("open_url", { url });
        } catch (e) {
          console.error("Failed to open Discord:", e);
        }
      });
    }

    if (openConfigBtn) {
      openConfigBtn.addEventListener("click", async () => {
        try {
          await invoke("open_config_folder");
        } catch (e) {
          console.error("Failed to open config folder:", e);
        }
      });
    }
  }

  private setupEventListeners() {
    // Version override checkbox listener (always available when visible)
    const versionOverrideCheckbox = document.getElementById("version-override") as HTMLInputElement;
    if (versionOverrideCheckbox) {
      versionOverrideCheckbox.addEventListener("change", async () => {
        if (!versionOverrideCheckbox.checked) return;
        try {
          this.config = await invoke<Config>("set_version_to_latest", { config: this.config });
          this.updateUI();

          // Close the modal and refresh the main UI
          const modal = document.getElementById("settings-modal")!;
          modal.style.display = "none";

          // Reload configuration to update UI state properly
          await this.loadConfiguration();

          this.setStatus(AppState.UP_TO_DATE, "Version synced to latest");
        } catch (error) {
          this.setStatus(AppState.ERROR, `Failed to update version: ${error}`);
          versionOverrideCheckbox.checked = false;
        }
      });
    }
    // Tauri event listeners
    listen<ProgressInfo>("download-progress", (event) => {
      this.updateProgress(event.payload);
    });

    listen<string>("download-complete", () => {
      this.onDownloadComplete();
    });

    listen<ErrorInfo>("download-error", (event) => {
      const errorInfo = event.payload;
      this.handleDownloadError(errorInfo);
    });

    listen<string>("status-update", (event) => {
      this.setStatus(AppState.INSTALLING, event.payload);
    });

    listen<ErrorInfo | string>("error", (event) => {
      const payload = event.payload;
      if (typeof payload === "string") {
        // Legacy string error handling
        this.handleLegacyError(payload);
      } else {
        // Enhanced error info handling
        this.handleErrorInfo(payload);
      }
    });

    listen("update-complete", () => {
      this.onUpdateComplete();
    });

    // UI event listeners
    this.updateButton.addEventListener("click", () => {
      if (this.currentStatus.state === AppState.UPDATE_AVAILABLE || this.currentStatus.state === AppState.FRESH_INSTALL_READY) {
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

      if (!this.config.installation_path) {
        this.isFirstRun = true;
        this.setStatus(AppState.NO_INSTALLATION, "No installation configured");
        this.showSettings(true);
      } else {
        // Check installation mode and path validity
        const mode = await invoke<InstallationMode>("detect_installation_mode", {
          path: this.config.installation_path,
          exeName: this.config.exe_path,
        });

        this.config.installation_mode = mode;

        if (mode === "fresh_install") {
          this.setStatus(AppState.FRESH_INSTALL_READY, "Ready for fresh installation");
          this.updateButton.textContent = "Install ReMakeplace";
          this.updateButton.disabled = false;
          this.updateButton.classList.add("btn-install");
        } else {
          this.checkForUpdates();
        }
      }
    } catch (error) {
      console.error("Failed to load configuration:", error);
      this.setStatus(AppState.ERROR, "Failed to load configuration");
    }
  }

  private async loadMetadata() {
    try {
      // Prefer GitHub raw metadata first
      const ts = Date.now();
      const githubUrl = `https://raw.githubusercontent.com/TrainerHol/RemakePlaceAutoupdater/refs/heads/main/metadata.json?cb=${ts}`;
      const gh = await fetch(githubUrl, { cache: "no-store" }).catch(() => null);
      if (gh && gh.ok) {
        this.metadata = await gh.json();
      } else {
        // Fallback to locally bundled metadata
        const local = await fetch(`/metadata.json?cb=${ts}`, { cache: "no-store" }).catch(() => null);
        if (local && local.ok) {
          this.metadata = await local.json();
        }
      }
    } catch (e) {
      console.warn("Failed to load metadata.json", e);
    } finally {
      this.renderMotd();
    }
  }

  private getDiscordInvite(): string {
    return this.metadata?.discordInvite?.trim() || "https://discord.gg/f2VAqXKWUw";
  }

  private renderMotd() {
    const motd = (this.metadata?.motd || "").trim();
    const motdLine = document.getElementById("motd-line") as HTMLElement | null;
    if (!motdLine) return;

    if (motd) {
      motdLine.textContent = motd;
      motdLine.style.display = "block";
    } else {
      motdLine.style.display = "none";
    }
  }

  private updateUI() {
    if (!this.config) return;

    this.currentVersionElement.textContent = this.config.current_version;
    this.installationPathElement.textContent = this.config.installation_path || "Not configured";

    // Update launch button state based on installation mode
    if (!this.config.installation_path) {
      this.launchButton.disabled = true;
    } else if (this.config.installation_mode === "fresh_install") {
      this.launchButton.disabled = true;
      this.launchButton.textContent = "Install Required";
    } else {
      this.launchButton.disabled = false;
      this.launchButton.textContent = "Launch ReMakeplace";
    }
  }

  private async checkForUpdates() {
    if (!this.config) return;

    this.setStatus(AppState.CHECKING_UPDATES, "Checking for updates...");
    this.updateButton.disabled = true;

    try {
      this.updateInfo = await invoke<UpdateInfo>("check_updates", { config: this.config });
      this.latestVersionElement.textContent = this.updateInfo.latest_version;

      if (this.config.installation_mode === "fresh_install") {
        this.setStatus(AppState.FRESH_INSTALL_READY, `Ready to install version ${this.updateInfo.latest_version}`);
        this.updateButton.textContent = "Install Now";
        this.updateButton.disabled = false;
        this.updateButton.classList.add("btn-install");
      } else if (this.updateInfo.is_available) {
        this.setStatus(AppState.UPDATE_AVAILABLE, `Update available: ${this.updateInfo.latest_version}`);
        this.updateButton.textContent = "Update Now";
        this.updateButton.disabled = false;
        this.updateButton.classList.add("btn-update");
      } else {
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
    if (!this.config || !this.updateInfo) return;

    // Check if this is a fresh install or update
    const isFreshInstall = this.config.installation_mode === "fresh_install";

    if (!isFreshInstall && !this.updateInfo.is_available) return;

    // Show confirmation dialog for fresh installs
    if (isFreshInstall) {
      const confirmed = await this.showConfirmation("Confirm Fresh Installation", `This will install ReMakeplace ${this.updateInfo.latest_version} to:\n\n${this.config.installation_path}\n\nDo you want to proceed?`);

      if (!confirmed) {
        return;
      }
    }

    const statusMessage = isFreshInstall ? "Starting fresh installation..." : "Starting download...";
    this.setStatus(AppState.DOWNLOADING, statusMessage);
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
    let statusText = `${progress.percentage.toFixed(1)}% - ${speedText}`;

    // Show retry information if applicable
    if (progress.is_retrying && progress.retry_count > 0) {
      statusText += ` (Retry ${progress.retry_count})`;
      if (progress.retry_reason) {
        statusText += ` - ${progress.retry_reason}`;
      }
    }

    this.progressText.textContent = statusText;

    const downloadMsg = progress.is_retrying ? `Retrying download... ${progress.percentage.toFixed(1)}%` : `Downloading update... ${progress.percentage.toFixed(1)}%`;

    this.setStatus(AppState.DOWNLOADING, downloadMsg);
  }

  private async onDownloadComplete() {
    if (!this.config || !this.updateInfo) return;

    const isFreshInstall = this.config.installation_mode === "fresh_install";
    const statusMessage = isFreshInstall ? "Download complete, starting fresh installation..." : "Download complete, starting installation...";

    this.setStatus(AppState.INSTALLING, statusMessage);

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
    const wasFreshInstall = this.config?.installation_mode === "fresh_install";
    const successMessage = wasFreshInstall ? "Fresh installation completed successfully!" : "Update completed successfully!";

    this.setStatus(AppState.UP_TO_DATE, successMessage);
    this.progressSection.style.display = "none";

    // Update installation mode and version after successful fresh install
    if (wasFreshInstall && this.config && this.updateInfo) {
      this.config.installation_mode = "update";
      this.config.current_version = this.updateInfo.latest_version;
      await invoke("save_config", { config: this.config });
    }

    // Reload configuration to get updated version and status
    await this.loadConfiguration();
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
    const versionOverrideGroup = document.getElementById("version-override-group")!;
    const versionOverrideCheckbox = document.getElementById("version-override") as HTMLInputElement;

    if (isFirstRun) {
      modalHeader.textContent = "Welcome to ReMakeplace Launcher";
      const modalBody = modal.querySelector(".modal-body")!;
      const existingWelcome = modalBody.querySelector(".welcome-message");
      if (!existingWelcome) {
        modalBody.insertAdjacentHTML("afterbegin", '<p class="welcome-message">Please select your ReMakeplace installation folder to continue.</p>');
      }
    } else {
      modalHeader.textContent = "Settings";
      const existingWelcome = modal.querySelector(".welcome-message");
      if (existingWelcome) {
        existingWelcome.remove();
      }
    }

    pathInput.value = this.config?.installation_path || "";

    // Show version override option for existing installations (not fresh install)
    if (this.config && this.config.installation_path && this.config.installation_mode === "update") {
      versionOverrideGroup.style.display = "block";
      versionOverrideCheckbox.checked = false;
    } else {
      versionOverrideGroup.style.display = "none";
    }

    modal.style.display = "flex";

    if (this.config?.installation_path) {
      this.validatePath(this.config.installation_path);
    }
  }

  private async validatePath(path: string) {
    const validation = document.getElementById("path-validation")!;
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;

    if (!path.trim()) {
      validation.innerHTML = "";
      validation.className = "validation-message";
      saveBtn.disabled = true;
      return;
    }

    // Show loading state
    validation.innerHTML = '<span class="validation-loading">🔄 Validating path...</span>';
    validation.className = "validation-message loading";
    saveBtn.disabled = true;

    try {
      // Detect installation mode
      const mode = await invoke<InstallationMode>("detect_installation_mode", {
        path: path,
        exeName: this.config?.exe_path || "Makeplace.exe",
      });

      // Get mode description
      const modeDescription = await invoke<string>("get_mode_description", {
        mode: mode,
      });

      // Validate with detailed error information
      try {
        await invoke<string>("validate_path_detailed", {
          path: path,
          exeName: this.config?.exe_path || "Makeplace.exe",
          mode: mode,
        });

        // Path is valid
        const modeText = mode === "fresh_install" ? "fresh installation" : "existing installation";
        validation.innerHTML = `
          <div class="validation-success">
            <span class="validation-icon">✅</span>
            <div class="validation-content">
              <div class="validation-main">Valid path for ${modeText}</div>
              <div class="validation-sub">${modeDescription}</div>
            </div>
          </div>
        `;
        validation.className = "validation-message valid";
        saveBtn.disabled = false;
      } catch (errorInfo: any) {
        // Path validation failed with detailed error
        this.showValidationError(validation, errorInfo);
        saveBtn.disabled = true;
      }
    } catch (error) {
      // Fallback for unexpected errors
      validation.innerHTML = `
        <div class="validation-error">
          <span class="validation-icon">❌</span>
          <div class="validation-content">
            <div class="validation-main">Error validating path</div>
            <div class="validation-sub">Please try again or select a different path</div>
          </div>
        </div>
      `;
      validation.className = "validation-message invalid";
      saveBtn.disabled = true;
    }
  }

  private showValidationError(validation: HTMLElement, errorInfo: ErrorInfo) {
    validation.innerHTML = `
      <div class="validation-error">
        <span class="validation-icon">❌</span>
        <div class="validation-content">
          <div class="validation-main">${errorInfo.user_message}</div>
          <div class="validation-sub">${errorInfo.recovery_suggestion}</div>
          ${errorInfo.category === ErrorCategory.Permission ? '<div class="validation-tip">💡 Try running as administrator</div>' : ""}
        </div>
      </div>
    `;
    validation.className = "validation-message invalid";
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
      // Detect and set installation mode
      const mode = await invoke<InstallationMode>("detect_installation_mode", {
        path: path,
        exeName: this.config.exe_path,
      });

      // Check if user is switching from an existing installation to a fresh install location
      const wasExistingInstall = this.config.installation_path && this.config.installation_mode === "update";
      const willBeFreshInstall = mode === "fresh_install";

      if (wasExistingInstall && willBeFreshInstall) {
        const confirmed = await this.showConfirmation("Fresh Installation", `The selected folder doesn't contain an existing ReMakeplace installation.\n\nDo you want to perform a fresh installation at:\n${path}`);

        if (!confirmed) {
          return;
        }
      }

      this.config.installation_path = path;
      this.config.installation_mode = mode;
      await invoke("save_config", { config: this.config });

      const modal = document.getElementById("settings-modal")!;
      modal.style.display = "none";

      this.updateUI();

      if (this.isFirstRun || mode === "fresh_install") {
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

  private handleDownloadError(errorInfo: ErrorInfo) {
    const message = errorInfo.is_retryable ? `${errorInfo.user_message} Retrying automatically...` : errorInfo.user_message;

    this.setStatus(AppState.ERROR, message);

    // Show detailed error in console for debugging
    console.error("Download error details:", errorInfo);

    // If it's retryable, don't hide progress section yet
    if (!errorInfo.is_retryable) {
      this.progressSection.style.display = "none";
      this.updateButton.disabled = false;
    }
  }

  private handleErrorInfo(errorInfo: ErrorInfo) {
    this.setStatus(AppState.ERROR, errorInfo.user_message);
    console.error("Error details:", errorInfo);
  }

  private handleLegacyError(errorMsg: string) {
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
  }

  private showConfirmation(title: string, message: string): Promise<boolean> {
    return new Promise((resolve) => {
      const modal = document.getElementById("confirmation-modal")!;
      const titleElement = document.getElementById("confirmation-title")!;
      const messageElement = document.getElementById("confirmation-message")!;
      const cancelBtn = document.getElementById("confirmation-cancel")!;
      const confirmBtn = document.getElementById("confirmation-confirm")!;

      titleElement.textContent = title;
      messageElement.textContent = message;
      modal.style.display = "flex";

      const cleanup = () => {
        modal.style.display = "none";
        cancelBtn.removeEventListener("click", handleCancel);
        confirmBtn.removeEventListener("click", handleConfirm);
        modal.removeEventListener("click", handleOutsideClick);
      };

      const handleCancel = () => {
        cleanup();
        resolve(false);
      };

      const handleConfirm = () => {
        cleanup();
        resolve(true);
      };

      const handleOutsideClick = (e: Event) => {
        if (e.target === modal) {
          handleCancel();
        }
      };

      cancelBtn.addEventListener("click", handleCancel);
      confirmBtn.addEventListener("click", handleConfirm);
      modal.addEventListener("click", handleOutsideClick);
    });
  }
}

// Initialize the application when DOM is loaded
document.addEventListener("DOMContentLoaded", () => {
  new ReMakeplaceUpdater();
});
