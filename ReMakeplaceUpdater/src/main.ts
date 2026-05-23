import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import BookOpen from "lucide/dist/esm/icons/book-open.mjs";
import CheckCircle from "lucide/dist/esm/icons/circle-check.mjs";
import CircleAlert from "lucide/dist/esm/icons/circle-alert.mjs";
import CircleX from "lucide/dist/esm/icons/circle-x.mjs";
import Download from "lucide/dist/esm/icons/download.mjs";
import Eraser from "lucide/dist/esm/icons/eraser.mjs";
import ExternalLink from "lucide/dist/esm/icons/external-link.mjs";
import FileText from "lucide/dist/esm/icons/file-text.mjs";
import Folder from "lucide/dist/esm/icons/folder.mjs";
import FolderOpen from "lucide/dist/esm/icons/folder-open.mjs";
import GalleryHorizontal from "lucide/dist/esm/icons/gallery-horizontal.mjs";
import House from "lucide/dist/esm/icons/house.mjs";
import Image from "lucide/dist/esm/icons/image.mjs";
import Info from "lucide/dist/esm/icons/info.mjs";
import LoaderCircle from "lucide/dist/esm/icons/loader-circle.mjs";
import MessageCircle from "lucide/dist/esm/icons/message-circle.mjs";
import Play from "lucide/dist/esm/icons/play.mjs";
import RefreshCw from "lucide/dist/esm/icons/refresh-cw.mjs";
import Save from "lucide/dist/esm/icons/save.mjs";
import Search from "lucide/dist/esm/icons/search.mjs";
import Settings from "lucide/dist/esm/icons/settings.mjs";
import Trash2 from "lucide/dist/esm/icons/trash-2.mjs";
import Wrench from "lucide/dist/esm/icons/wrench.mjs";
import X from "lucide/dist/esm/icons/x.mjs";
import type { Config, UpdateInfo, ProgressInfo, AppStatus, InstallationDetection, ErrorInfo, Metadata } from "./types";
import { AppState, ErrorCategory } from "./types";

type ProgressStage = "idle" | "download" | "extract" | "validate" | "install" | "complete" | "error";
type IconNode = Array<[string, Record<string, string | number>, IconNode?]>;

const LUCIDE_ICON_NODES: Record<string, IconNode> = {
  "book-open": BookOpen,
  "check-circle-2": CheckCircle,
  "circle-alert": CircleAlert,
  "circle-x": CircleX,
  download: Download,
  eraser: Eraser,
  "external-link": ExternalLink,
  "file-text": FileText,
  folder: Folder,
  "folder-open": FolderOpen,
  "gallery-horizontal": GalleryHorizontal,
  house: House,
  image: Image,
  info: Info,
  "loader-circle": LoaderCircle,
  "message-circle": MessageCircle,
  play: Play,
  "refresh-cw": RefreshCw,
  save: Save,
  search: Search,
  settings: Settings,
  "trash-2": Trash2,
  wrench: Wrench,
  x: X,
};

class ReMakeplaceUpdater {
  private config: Config | null = null;
  private updateInfo: UpdateInfo | null = null;
  private installationDetection: InstallationDetection | null = null;
  private currentStatus: AppStatus = {
    state: AppState.IDLE,
    message: "Initializing...",
  };
  private isFirstRun = false;
  private metadata: Metadata | null = null;
  private actionsBusy = false;

  // UI Elements
  private statusMessage!: HTMLElement;
  private currentVersionElement!: HTMLElement;
  private latestVersionElement!: HTMLElement;
  private installationPathElement!: HTMLElement;
  private validationStatusElement!: HTMLElement;
  private progressBar!: HTMLElement;
  private progressText!: HTMLElement;
  private progressTitle!: HTMLElement;
  private progressPercent!: HTMLElement;
  private progressSize!: HTMLElement;
  private progressSpeed!: HTMLElement;
  private updateButton!: HTMLButtonElement;
  private launchButton!: HTMLButtonElement;
  private settingsButton!: HTMLButtonElement;
  private clearCacheButton!: HTMLButtonElement;
  private openCustomButton!: HTMLButtonElement;
  private openSaveButton!: HTMLButtonElement;
  private progressSection!: HTMLElement;
  private progressStage: ProgressStage = "idle";

  constructor() {
    this.initializeUI();
    this.setupEventListeners();
    this.loadConfiguration();
    this.loadMetadata();
    this.setupDeepLinkListener();
  }

  private initializeUI() {
    document.body.innerHTML = `
      <div class="app-container">
        <!-- Header Section -->
        <div class="header">
          <h1>ReMakeplace Autoupdater</h1>
          <div class="tabs">
            <button id="tab-updates" class="tab active"><i data-lucide="refresh-cw"></i><span>Updates</span></button>
            <button id="tab-gallery" class="tab"><i data-lucide="gallery-horizontal"></i><span>Gallery</span></button>
          </div>
        </div>

        <!-- Content Wrapper -->
        <div class="content-wrapper">
          <div id="view-updates">
          <!-- Installation Path Section -->
          <div class="section">
            <div class="path-display">
              <span class="path-label">Installation Path:</span>
              <span id="installation-path" class="path-text">Not configured</span>
              <button id="settings-btn" class="settings-btn"><i data-lucide="settings"></i><span>Settings</span></button>
            </div>
            <div id="validation-status" class="validation-status">Not verified</div>
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
            <div class="progress-container" data-phase="idle">
              <div class="progress-header">
                <span class="progress-mark"><i data-lucide="download"></i></span>
                <div class="progress-copy">
                  <div id="progress-title" class="progress-title">Preparing update</div>
                  <div id="progress-text" class="progress-detail">Waiting for download to begin</div>
                </div>
                <div id="progress-percent" class="progress-percent">0%</div>
              </div>
              <div id="progress-bar" class="progress-bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0">
                <div class="progress-fill"></div>
              </div>
              <div class="progress-meta">
                <span id="progress-size">0 MB downloaded</span>
                <span id="progress-speed">0.0 MB/s</span>
              </div>
              <div class="progress-steps" aria-hidden="true">
                <span class="progress-step" data-step="download"><span class="progress-dot"></span>Download</span>
                <span class="progress-step" data-step="extract"><span class="progress-dot"></span>Extract</span>
                <span class="progress-step" data-step="validate"><span class="progress-dot"></span>Validate</span>
                <span class="progress-step" data-step="install"><span class="progress-dot"></span>Install</span>
              </div>
            </div>
          </div>

          <!-- Button Section -->
          <div class="section button-section">
            <div class="action-row action-primary-row">
              <button id="update-btn" class="btn btn-primary" disabled><i data-lucide="search"></i><span>Check for Updates</span></button>
              <button id="launch-btn" class="btn btn-secondary"><i data-lucide="play"></i><span>Launch</span></button>
            </div>
            <div class="action-row action-utility-row">
              <button id="open-custom-btn" class="btn btn-small protected-folder-action" hidden><i data-lucide="house"></i><span>Open Custom</span></button>
              <button id="open-save-btn" class="btn btn-small protected-folder-action" hidden><i data-lucide="save"></i><span>Open Save</span></button>
              <button id="clear-cache-btn" class="btn btn-small" title="Cleans up leftover downloaded update files."><i data-lucide="eraser"></i><span>Clear Cache</span></button>
            </div>
          </div>
          </div>
          <div id="view-gallery" style="display:none;">
            <div class="section">
              <div class="gallery-header">
                <h2 class="gallery-title">Your Designs</h2>
                <button id="download-designs-btn" class="btn btn-primary btn-small"><i data-lucide="download"></i><span>Download Designs</span></button>
              </div>
              <div id="gallery-grid" class="gallery-grid"></div>
              <div id="gallery-empty" class="gallery-empty" style="display:none;">
                <div class="empty-illustration"><i data-lucide="image"></i></div>
                <div class="empty-title">No designs yet</div>
                <div class="empty-sub">Use the Open in ReMakeplace Autoupdater button on ffxivhousing.com to send designs here, or browse and download from the website.</div>
                <button id="download-designs-btn-empty" class="btn btn-primary"><i data-lucide="download"></i><span>Download Designs</span></button>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer Section -->
        <div class="footer">
          <div class="footer-left" id="readme-link" title="Please read the README before asking questions.">
            <span class="icon book" aria-hidden="true">
              <i data-lucide="book-open"></i>
            </span>
            <span class="footer-text">Read README first</span>
          </div>
          <div id="motd-line" class="motd-line" style="display: none;"></div>
          <div class="footer-right" id="discord-link" title="Join the Discord">
            <span class="icon discord" aria-hidden="true">
              <i data-lucide="message-circle"></i>
            </span>
            <span class="footer-text">Join the Discord</span>
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
                  <button id="browse-btn" class="btn btn-small"><i data-lucide="folder-open"></i><span>Browse</span></button>
                </div>
                <div id="path-validation" class="validation-message"></div>
              </div>
              <div class="form-group" id="version-override-group" style="display: none;">
                <label>
                  <input type="checkbox" id="version-override" />
                  Set current version to latest
                </label>
                <div class="help-text" title="If your installation shows version 0.0.0 but is actually up to date, check this to sync with the latest version without reinstalling."><i data-lucide="info"></i><span>For existing installations showing incorrect version</span></div>
              </div>
              <div class="form-group">
                <button id="open-config-btn" class="btn btn-small" title="Open the folder where config.json is stored"><i data-lucide="folder"></i><span>Open config folder</span></button>
                <button id="settings-open-custom-btn" class="btn btn-small protected-folder-action" hidden title="Open the Custom folder for the selected installation"><i data-lucide="house"></i><span>Open Custom</span></button>
                <button id="settings-open-save-btn" class="btn btn-small protected-folder-action" hidden title="Open the Save folder for the selected installation"><i data-lucide="save"></i><span>Open Save</span></button>
                <button id="verify-repair-btn" class="btn btn-small btn-secondary" hidden title="Re-check the selected ReMakeplace folder and prepare repair if files are missing"><i data-lucide="wrench"></i><span>Verify / Repair</span></button>
              </div>
            </div>
            <div class="modal-footer">
              <button id="cancel-btn" class="btn btn-secondary"><i data-lucide="x"></i><span>Cancel</span></button>
              <button id="save-btn" class="btn btn-primary" disabled><i data-lucide="check-circle-2"></i><span>Save & Continue</span></button>
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
              <button id="confirmation-cancel" class="btn btn-secondary"><i data-lucide="x"></i><span>Cancel</span></button>
              <button id="confirmation-confirm" class="btn btn-primary"><i data-lucide="check-circle-2"></i><span>Confirm</span></button>
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
    this.validationStatusElement = document.getElementById("validation-status")!;
    this.progressBar = document.getElementById("progress-bar")!;
    this.progressText = document.getElementById("progress-text")!;
    this.progressTitle = document.getElementById("progress-title")!;
    this.progressPercent = document.getElementById("progress-percent")!;
    this.progressSize = document.getElementById("progress-size")!;
    this.progressSpeed = document.getElementById("progress-speed")!;
    this.updateButton = document.getElementById("update-btn") as HTMLButtonElement;
    this.launchButton = document.getElementById("launch-btn") as HTMLButtonElement;
    this.settingsButton = document.getElementById("settings-btn") as HTMLButtonElement;
    this.clearCacheButton = document.getElementById("clear-cache-btn") as HTMLButtonElement;
    this.openCustomButton = document.getElementById("open-custom-btn") as HTMLButtonElement;
    this.openSaveButton = document.getElementById("open-save-btn") as HTMLButtonElement;
    this.progressSection = document.getElementById("progress-section")!;
    // Footer and settings helpers
    const readmeLink = document.getElementById("readme-link") as HTMLElement | null;
    const discordLink = document.getElementById("discord-link") as HTMLElement | null;
    const openConfigBtn = document.getElementById("open-config-btn") as HTMLButtonElement | null;
    const settingsOpenCustomBtn = document.getElementById("settings-open-custom-btn") as HTMLButtonElement | null;
    const settingsOpenSaveBtn = document.getElementById("settings-open-save-btn") as HTMLButtonElement | null;
    const verifyRepairBtn = document.getElementById("verify-repair-btn") as HTMLButtonElement | null;
    const tabUpdates = document.getElementById("tab-updates") as HTMLButtonElement | null;
    const tabGallery = document.getElementById("tab-gallery") as HTMLButtonElement | null;
    const viewUpdates = document.getElementById("view-updates") as HTMLElement | null;
    const viewGallery = document.getElementById("view-gallery") as HTMLElement | null;

    if (readmeLink) {
      readmeLink.addEventListener("click", async () => {
        try {
          await invoke("open_url", { url: "https://github.com/TrainerHol/RemakePlaceAutoupdater#remakeplace-autoupdater" });
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

    settingsOpenCustomBtn?.addEventListener("click", () => this.openGameDataFolder("custom"));
    settingsOpenSaveBtn?.addEventListener("click", () => this.openGameDataFolder("save"));
    verifyRepairBtn?.addEventListener("click", async () => {
      const pathInput = document.getElementById("path-input") as HTMLInputElement;
      await this.validatePath(pathInput.value);
      if (this.config?.installation_path) {
        await this.loadConfiguration();
      }
    });

    if (tabUpdates && tabGallery && viewUpdates && viewGallery) {
      tabUpdates.addEventListener("click", () => {
        tabUpdates.classList.add("active");
        tabGallery.classList.remove("active");
        viewUpdates.style.display = "flex";
        viewGallery.style.display = "none";
      });
      tabGallery.addEventListener("click", async () => {
        tabGallery.classList.add("active");
        tabUpdates.classList.remove("active");
        viewUpdates.style.display = "none";
        viewGallery.style.display = "block";
        await this.loadGallery();
      });
    }

    this.renderIcons();
  }

  private renderIcons() {
    document.querySelectorAll<HTMLElement>("[data-lucide]").forEach((element) => {
      if (element.tagName.toLowerCase() === "svg") return;

      const iconName = element.getAttribute("data-lucide");
      if (!iconName) return;

      const iconNode = LUCIDE_ICON_NODES[iconName];
      if (!iconNode) return;

      const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
      svg.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      svg.setAttribute("width", "24");
      svg.setAttribute("height", "24");
      svg.setAttribute("viewBox", "0 0 24 24");
      svg.setAttribute("fill", "none");
      svg.setAttribute("stroke", "currentColor");
      svg.setAttribute("stroke-width", "2");
      svg.setAttribute("stroke-linecap", "round");
      svg.setAttribute("stroke-linejoin", "round");
      svg.setAttribute("data-lucide", iconName);
      svg.setAttribute("aria-hidden", element.getAttribute("aria-hidden") || "true");
      svg.classList.add("lucide", `lucide-${iconName}`);

      this.appendIconNodes(svg, iconNode);
      element.replaceWith(svg);
    });
  }

  private appendIconNodes(parent: SVGElement, nodes: IconNode) {
    nodes.forEach((node) => {
      const tag = node[0];
      const attrs = node[1];
      const children = node[2];
      const child = document.createElementNS("http://www.w3.org/2000/svg", tag);

      Object.keys(attrs).forEach((name) => {
        child.setAttribute(name, String(attrs[name]));
      });

      if (children) {
        this.appendIconNodes(child, children);
      }

      parent.appendChild(child);
    });
  }

  private icon(name: string): string {
    return `<i data-lucide="${name}" aria-hidden="true"></i>`;
  }

  private setButtonContent(button: HTMLButtonElement, iconName: string, label: string) {
    button.innerHTML = `${this.icon(iconName)}<span>${this.escapeHtml(label)}</span>`;
    this.renderIcons();
  }

  private hasVerifiedInstallation(): boolean {
    return !!this.config?.installation_path.trim() && this.installationDetection?.status === "existing_valid";
  }

  private shouldShowVerifyRepair(path: string, detection: InstallationDetection | null): boolean {
    if (!path.trim() || !detection) return false;
    if (this.isFirstRun) return false;
    if (!this.config?.installation_path.trim()) return false;
    if (path.trim() !== this.config.installation_path.trim()) return false;

    return detection.status === "existing_valid" || detection.status === "existing_incomplete";
  }

  private updateVerifyRepairVisibility(detection: InstallationDetection | null = null) {
    const verifyRepairBtn = document.getElementById("verify-repair-btn") as HTMLButtonElement | null;
    const pathInput = document.getElementById("path-input") as HTMLInputElement | null;
    if (!verifyRepairBtn || !pathInput) return;

    const shouldShow = this.shouldShowVerifyRepair(pathInput.value, detection);
    verifyRepairBtn.hidden = !shouldShow;
    verifyRepairBtn.disabled = !shouldShow;
  }

  private escapeHtml(value: unknown): string {
    return String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  private escapeAttribute(value: unknown): string {
    return this.escapeHtml(value);
  }

  private formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return "0 MB";

    const units = ["B", "KB", "MB", "GB"];
    let value = bytes;
    let unitIndex = 0;

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }

    const decimals = unitIndex <= 1 ? 0 : 1;
    return `${value.toFixed(decimals)} ${units[unitIndex]}`;
  }

  private getProgressIcon(stage: ProgressStage): string {
    switch (stage) {
      case "download":
        return "download";
      case "extract":
        return "folder-open";
      case "validate":
        return "check-circle-2";
      case "install":
        return "wrench";
      case "complete":
        return "check-circle-2";
      case "error":
        return "circle-x";
      case "idle":
      default:
        return "download";
    }
  }

  private setProgressBar(percent: number) {
    const clamped = Math.max(0, Math.min(100, Number.isFinite(percent) ? percent : 0));
    const progressFill = this.progressBar.querySelector(".progress-fill") as HTMLElement;
    progressFill.style.width = `${clamped}%`;
    this.progressBar.setAttribute("aria-valuenow", clamped.toFixed(0));
    this.progressPercent.textContent = `${clamped.toFixed(0)}%`;
  }

  private setProgressStage(stage: ProgressStage, title: string, detail: string) {
    this.progressSection.style.display = "block";
    const progressContainer = this.progressSection.querySelector(".progress-container") as HTMLElement;
    progressContainer.dataset.phase = stage;
    this.progressTitle.textContent = title;
    this.progressText.textContent = detail;

    if (this.progressStage !== stage) {
      const mark = this.progressSection.querySelector(".progress-mark") as HTMLElement;
      mark.innerHTML = this.icon(this.getProgressIcon(stage));
      this.progressStage = stage;
      this.renderIcons();
    }

    this.updateProgressSteps(stage);
    this.updateStatusVisibility();
  }

  private resetProgress(title: string, detail: string) {
    this.progressStage = "idle";
    this.setProgressBar(0);
    this.progressSize.textContent = "0 MB downloaded";
    this.progressSpeed.textContent = "0.0 MB/s";
    this.setProgressStage("download", title, detail);
  }

  private updateStatusVisibility() {
    const progressVisible = this.progressSection.style.display !== "none";
    this.statusMessage.hidden = progressVisible;
  }

  private setActionsBusy(isBusy: boolean) {
    this.actionsBusy = isBusy;
    this.updateUI();
  }

  private updateProgressSteps(stage: ProgressStage) {
    const order: ProgressStage[] = ["download", "extract", "validate", "install"];
    const activeIndex = stage === "complete" ? order.length : order.indexOf(stage);

    this.progressSection.querySelectorAll<HTMLElement>(".progress-step").forEach((step) => {
      const stepStage = step.dataset.step as ProgressStage | undefined;
      const stepIndex = stepStage ? order.indexOf(stepStage) : -1;
      step.classList.toggle("is-done", stage === "complete" || (stepIndex >= 0 && stepIndex < activeIndex));
      step.classList.toggle("is-active", stepIndex === activeIndex && stage !== "complete" && stage !== "error");
    });
  }

  private applyInstallProgress(message: string) {
    const normalized = message.toLowerCase();

    if (normalized.includes("extract") || normalized.includes("7z archive")) {
      const match = message.match(/\((\d{1,3})%\)$/);
      if (match) {
        this.setProgressBar(Number(match[1]));
      } else if (normalized.includes("preparing") || normalized.includes("reading")) {
        this.setProgressBar(0);
      }
      this.setProgressStage("extract", "Extracting archive", message);
      return;
    }

    if (normalized.includes("validat")) {
      this.setProgressBar(100);
      this.setProgressStage("validate", "Validating files", message);
      return;
    }

    if (normalized.includes("backup")) {
      this.setProgressStage("install", "Preserving user data", message);
      return;
    }

    if (normalized.includes("restore")) {
      this.setProgressStage("install", "Restoring user data", message);
      return;
    }

    if (normalized.includes("clean")) {
      this.setProgressStage("install", "Cleaning up", message);
      return;
    }

    if (normalized.includes("complete")) {
      this.setProgressBar(100);
      this.setProgressStage("complete", "Update complete", message);
      return;
    }

    this.setProgressStage("install", "Installing files", message);
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
      this.applyInstallProgress(event.payload);
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
      if (this.currentStatus.state === AppState.UPDATE_AVAILABLE || this.currentStatus.state === AppState.FRESH_INSTALL_READY || this.currentStatus.state === AppState.REPAIR_READY) {
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

    this.openCustomButton.addEventListener("click", () => {
      this.openGameDataFolder("custom");
    });

    this.openSaveButton.addEventListener("click", () => {
      this.openGameDataFolder("save");
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
      this.installationDetection = null;
      this.updateUI();

      if (!this.config.installation_path) {
        this.isFirstRun = true;
        this.setStatus(AppState.NO_INSTALLATION, "No installation configured");
        this.showSettings(true);
      } else {
        const detection = await invoke<InstallationDetection>("detect_installation", {
          path: this.config.installation_path,
          exeName: this.config.exe_path,
        });

        this.installationDetection = detection;
        this.config.installation_mode = detection.mode;
        this.config.installation_path = detection.normalized_path || this.config.installation_path;
        this.updateUI();

        if (detection.status === "invalid_path") {
          this.setStatus(AppState.ERROR, detection.message);
          this.setButtonContent(this.updateButton, "folder-open", "Fix Folder");
          this.updateButton.disabled = true;
        } else if (detection.status === "fresh_empty") {
          await this.checkForUpdates();
        } else if (detection.status === "existing_incomplete") {
          await this.checkForUpdates();
        } else {
          await this.checkForUpdates();
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

  private async loadGallery() {
    try {
      const items = await invoke<any>("list_gallery");
      const grid = document.getElementById("gallery-grid") as HTMLElement | null;
      const empty = document.getElementById("gallery-empty") as HTMLElement | null;
      const headerBtn = document.getElementById("download-designs-btn") as HTMLButtonElement | null;
      const emptyBtn = document.getElementById("download-designs-btn-empty") as HTMLButtonElement | null;
      if (!grid) return;
      const list = items as Array<any>;
      grid.innerHTML = list
        .map((it) => {
          const src = it.image_path ? convertFileSrc(it.image_path) : null;
          const img = src ? `<img src="${this.escapeAttribute(src)}" alt="" class="thumb" crossorigin="anonymous"/>` : `<div class="thumb placeholder">${this.icon("image")}</div>`;
          return `
          <div class="card">
            <div class="thumb-wrap">${img}</div>
            <div class="meta">
              <div class="title">${this.escapeHtml(it.title)}</div>
              <div class="sub">${this.escapeHtml(it.kind)} - ${this.escapeHtml(it.author)}</div>
            </div>
            <div class="actions">
              <button data-json="${this.escapeAttribute(it.json_path)}" class="btn btn-small open-folder">${this.icon("external-link")}<span>Show in Folder</span></button>
              <button data-id="${this.escapeAttribute(it.id)}" class="btn btn-small btn-secondary delete-entry">${this.icon("trash-2")}<span>Delete</span></button>
            </div>
          </div>
        `;
        })
        .join("");
      this.renderIcons();

      // Wire actions
      grid.querySelectorAll(".open-folder").forEach((btn) => {
        btn.addEventListener("click", async (e) => {
          const el = e.currentTarget as HTMLElement;
          const jsonPath = el.getAttribute("data-json");
          if (!jsonPath) return;
          try {
            await invoke("reveal_path", { path: jsonPath });
          } catch (err) {
            console.error("Failed to open path:", err);
          }
        });
      });

      // Wire delete actions
      grid.querySelectorAll(".delete-entry").forEach((btn) => {
        btn.addEventListener("click", async (e) => {
          const el = e.currentTarget as HTMLElement;
          const id = el.getAttribute("data-id");
          if (!id) return;
          try {
            await invoke("delete_gallery_entry", { id });
            await this.loadGallery();
          } catch (err) {
            console.error("Failed to delete entry:", err);
          }
        });
      });

      const openWebsite = async () => {
        try {
          await invoke("open_url", { url: "https://ffxivhousing.com" });
        } catch (e) {
          console.error("Failed to open website:", e);
        }
      };

      if (headerBtn) headerBtn.onclick = openWebsite;
      if (emptyBtn) emptyBtn.onclick = openWebsite;

      if (empty) {
        if (!list || list.length === 0) {
          empty.style.display = "flex";
          grid.style.display = "none";
        } else {
          empty.style.display = "none";
          grid.style.display = "grid";
        }
      }

      // Fallback: if an image fails, replace with data URL read via backend
      grid.querySelectorAll<HTMLImageElement>("img.thumb").forEach((imgEl) => {
        const fallback = async () => {
          const card = imgEl.closest(".card");
          const idx = card ? Array.from(grid.children).indexOf(card as Element) : -1;
          const item = idx >= 0 ? list[idx] : null;
          if (item?.image_path) {
            try {
              const dataUrl = await invoke<string>("get_image_data_url", { path: item.image_path });
              imgEl.src = dataUrl;
            } catch {}
          }
        };

        imgEl.addEventListener("error", () => {
          void fallback();
        });

        if (imgEl.complete && imgEl.naturalWidth === 0) {
          void fallback();
        }
      });
    } catch (e) {
      console.error("Failed to load gallery:", e);
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
      const content = this.escapeHtml(motd);
      motdLine.setAttribute("aria-label", motd);
      motdLine.innerHTML = `
        <div class="motd-track">
          <span class="motd-item">${content}</span>
          <span class="motd-item" aria-hidden="true">${content}</span>
        </div>
      `;
      motdLine.style.display = "flex";
    } else {
      motdLine.innerHTML = "";
      motdLine.removeAttribute("aria-label");
      motdLine.style.display = "none";
    }
  }

  private updateUI() {
    if (!this.config) return;

    this.currentVersionElement.textContent = this.config.current_version;
    this.currentVersionElement.classList.toggle("version-unverified", !!this.config.installation_path && this.installationDetection?.status !== "existing_valid");
    this.currentVersionElement.title = this.installationDetection?.status === "existing_valid"
      ? "Version recorded for the selected installation"
      : "Stored version is not verified against the selected folder";
    this.installationPathElement.textContent = this.config.installation_path || "Not configured";

    if (!this.installationDetection) {
      this.validationStatusElement.textContent = this.config.installation_path ? "Not verified" : "Choose an installation folder";
      this.validationStatusElement.className = "validation-status";
    } else {
      this.validationStatusElement.textContent = this.installationDetection.message;
      this.validationStatusElement.className = `validation-status ${this.installationDetection.status}`;
    }

    const canOpenDataFolders = !this.actionsBusy && this.hasVerifiedInstallation();
    this.openCustomButton.hidden = !canOpenDataFolders;
    this.openSaveButton.hidden = !canOpenDataFolders;
    this.openCustomButton.disabled = !canOpenDataFolders;
    this.openSaveButton.disabled = !canOpenDataFolders;
    this.clearCacheButton.hidden = this.actionsBusy;
    this.clearCacheButton.disabled = this.actionsBusy;

    // Update launch button state based on installation mode
    if (this.actionsBusy) {
      this.launchButton.hidden = true;
      this.launchButton.disabled = true;
    } else if (!this.config.installation_path) {
      this.launchButton.hidden = true;
      this.launchButton.disabled = true;
    } else if (this.installationDetection?.status === "existing_incomplete") {
      this.launchButton.hidden = true;
      this.launchButton.disabled = true;
      this.setButtonContent(this.launchButton, "wrench", "Repair Required");
    } else if (this.config.installation_mode === "fresh_install") {
      this.launchButton.hidden = true;
      this.launchButton.disabled = true;
      this.setButtonContent(this.launchButton, "download", "Install Required");
    } else {
      this.launchButton.hidden = false;
      this.launchButton.disabled = false;
      this.setButtonContent(this.launchButton, "play", "Launch");
    }
  }

  private async setupDeepLinkListener() {
    try {
      await onOpenUrl(async (urls: string[]) => {
        const url = urls[0];
        try {
          await invoke("handle_deep_link", { url });
          // Switch to Gallery and refresh after import
          const tabUpdates = document.getElementById("tab-updates") as HTMLButtonElement | null;
          const tabGallery = document.getElementById("tab-gallery") as HTMLButtonElement | null;
          const viewUpdates = document.getElementById("view-updates") as HTMLElement | null;
          const viewGallery = document.getElementById("view-gallery") as HTMLElement | null;
          if (tabGallery && tabUpdates && viewUpdates && viewGallery) {
            tabGallery.classList.add("active");
            tabUpdates.classList.remove("active");
            viewUpdates.style.display = "none";
            viewGallery.style.display = "block";
          }
          await this.loadGallery();
        } catch (e) {
          console.error("Failed to import from deep link:", e);
        }
      });
    } catch (e) {
      console.warn("Deep link plugin not available:", e);
    }
  }

  private async checkForUpdates() {
    if (!this.config) return;

    this.setStatus(AppState.CHECKING_UPDATES, "Checking for updates...");
    this.updateButton.hidden = false;
    this.updateButton.disabled = true;
    this.updateButton.classList.remove("is-busy");

    try {
      this.updateInfo = await invoke<UpdateInfo>("check_updates", { config: this.config });
      this.latestVersionElement.textContent = this.updateInfo.latest_version;
      this.updateButton.classList.remove("btn-update", "btn-install");

      if (!this.updateInfo.download_url) {
        this.setStatus(AppState.ERROR, "No supported ReMakeplace archive was found on the latest release");
        this.updateButton.hidden = false;
        this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
        this.updateButton.disabled = false;
      } else if (this.installationDetection?.status === "fresh_empty" || this.config.installation_mode === "fresh_install") {
        this.setStatus(AppState.FRESH_INSTALL_READY, `Ready to install version ${this.updateInfo.latest_version}`);
        this.updateButton.hidden = false;
        this.setButtonContent(this.updateButton, "download", "Install Now");
        this.updateButton.disabled = false;
        this.updateButton.classList.add("btn-install");
      } else if (this.installationDetection?.status === "existing_incomplete") {
        this.setStatus(AppState.REPAIR_READY, `Repair available using version ${this.updateInfo.latest_version}`);
        this.updateButton.hidden = false;
        this.setButtonContent(this.updateButton, "wrench", "Repair Install");
        this.updateButton.disabled = false;
        this.updateButton.classList.add("btn-install");
      } else if (this.updateInfo.is_available) {
        this.setStatus(AppState.UPDATE_AVAILABLE, `Update available: ${this.updateInfo.latest_version}`);
        this.updateButton.hidden = false;
        this.setButtonContent(this.updateButton, "download", "Update Now");
        this.updateButton.disabled = false;
        this.updateButton.classList.add("btn-update");
      } else {
        this.setStatus(AppState.UP_TO_DATE, "You have the latest version");
        this.setButtonContent(this.updateButton, "check-circle-2", "Up to Date");
        this.updateButton.hidden = true;
        this.updateButton.disabled = true;
        this.updateButton.classList.remove("btn-update");
      }
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to check updates: ${error}`);
      this.updateButton.hidden = false;
      this.updateButton.disabled = false;
      this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
    }
  }

  private async startUpdate() {
    if (!this.config || !this.updateInfo) return;

    // Check if this is a fresh install or update
    const isFreshInstall = this.config.installation_mode === "fresh_install";
    const isRepair = this.installationDetection?.status === "existing_incomplete";

    if (!this.updateInfo.download_url) {
      this.setStatus(AppState.ERROR, "No supported ReMakeplace archive is available to download");
      return;
    }

    if (!isFreshInstall && !isRepair && !this.updateInfo.is_available) return;

    // Show confirmation dialog for fresh installs
    if (isFreshInstall) {
      const confirmed = await this.showConfirmation("Confirm Fresh Installation", `This will install ReMakeplace ${this.updateInfo.latest_version} to:\n\n${this.config.installation_path}\n\nDo you want to proceed?`);

      if (!confirmed) {
        return;
      }
    } else if (isRepair) {
      const detail = this.installationDetection?.details?.join("\n") || "The selected installation is missing required files.";
      const confirmed = await this.showConfirmation("Repair Installation", `This will download ReMakeplace ${this.updateInfo.latest_version} and repair missing game files at:\n\n${this.config.installation_path}\n\n${detail}\n\nYour Custom and Save folders will be preserved.`);

      if (!confirmed) {
        return;
      }
    }

    const statusMessage = isFreshInstall ? "Starting fresh installation..." : isRepair ? "Starting repair..." : "Starting download...";
    this.setStatus(AppState.DOWNLOADING, statusMessage);
    this.setActionsBusy(true);
    this.updateButton.hidden = false;
    this.resetProgress(
      isFreshInstall ? "Installing ReMakeplace" : isRepair ? "Repairing installation" : "Updating ReMakeplace",
      "Connecting to the release archive"
    );
    this.updateButton.disabled = true;
    this.updateButton.classList.add("is-busy");
    this.setButtonContent(this.updateButton, "loader-circle", "Working");

    try {
      const filename = this.updateInfo.asset_name || this.updateInfo.download_url.split("/").pop() || "update.7z";

      await invoke("start_download", {
        url: this.updateInfo.download_url,
        version: this.updateInfo.latest_version,
        originalFilename: filename,
        expectedSize: this.updateInfo.asset_size,
      });
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to start download: ${error}`);
      this.setProgressStage("error", "Download could not start", String(error));
      this.setActionsBusy(false);
      this.updateButton.classList.remove("is-busy");
      this.updateButton.hidden = false;
      this.updateButton.disabled = false;
      this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
    }
  }

  private updateProgress(progress: ProgressInfo) {
    if (!(progress.is_retrying && progress.percentage <= 0)) {
      this.setProgressBar(progress.percentage);
    }

    const speedText = progress.speed > 0 ? `${progress.speed.toFixed(1)} MB/s` : "0.0 MB/s";
    this.progressSpeed.textContent = speedText;
    const sizeText = progress.total > 0
      ? `${this.formatBytes(progress.downloaded)} of ${this.formatBytes(progress.total)}`
      : `${this.formatBytes(progress.downloaded)} downloaded`;
    this.progressSize.textContent = sizeText;

    let progressDetail = `${sizeText} at ${speedText}`;

    // Show retry information if applicable
    if (progress.is_retrying && progress.retry_count > 0) {
      progressDetail = `Retry ${progress.retry_count} after an interrupted connection`;
    }

    this.setProgressStage("download", progress.is_retrying ? "Retrying download" : "Downloading release", progressDetail);

    const downloadMsg = progress.is_retrying ? "Retrying download" : "Downloading release";

    this.setStatus(AppState.DOWNLOADING, downloadMsg);
  }

  private async onDownloadComplete() {
    if (!this.config || !this.updateInfo) return;

    const isFreshInstall = this.config.installation_mode === "fresh_install";
    const isRepair = this.installationDetection?.status === "existing_incomplete";
    const statusMessage = isFreshInstall ? "Download complete, starting fresh installation..." : isRepair ? "Download complete, starting repair..." : "Download complete, starting installation...";

    this.setStatus(AppState.INSTALLING, statusMessage);
    this.setProgressBar(100);
    this.setProgressStage("extract", "Preparing files", "Download complete. Extracting to a staging folder.");

    try {
      const filename = this.updateInfo.asset_name || this.updateInfo.download_url.split("/").pop() || "update.7z";

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
      console.error("Failed to start installation:", error);
      const message = "The update could not be installed. Try clearing the cache and downloading again.";
      this.setStatus(AppState.ERROR, message);
      this.setProgressStage("error", "Installation stopped", message);
      this.setActionsBusy(false);
      this.updateButton.classList.remove("is-busy");
      this.updateButton.hidden = false;
      this.updateButton.disabled = false;
      this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
    }
  }

  private async onUpdateComplete() {
    const wasFreshInstall = this.config?.installation_mode === "fresh_install";
    const successMessage = wasFreshInstall ? "Fresh installation completed successfully!" : "Update completed successfully!";

    this.setProgressBar(100);
    this.setProgressStage("complete", "Update complete", "ReMakeplace is ready to launch.");
    this.setStatus(AppState.UP_TO_DATE, successMessage);
    this.setActionsBusy(false);
    this.updateButton.classList.remove("is-busy");
    window.setTimeout(() => {
      if (this.currentStatus.state === AppState.UP_TO_DATE) {
        this.progressSection.style.display = "none";
        this.updateStatusVisibility();
      }
    }, 1200);

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
      modalHeader.textContent = "Welcome to ReMakeplace Autoupdater";
      const modalBody = modal.querySelector(".modal-body")!;
      const existingWelcome = modalBody.querySelector(".welcome-message");
      if (!existingWelcome) {
        modalBody.insertAdjacentHTML("afterbegin", '<p class="welcome-message">Select an existing ReMakeplace folder to update, or choose an empty folder for a fresh install.</p>');
      }
    } else {
      modalHeader.textContent = "Settings";
      const existingWelcome = modal.querySelector(".welcome-message");
      if (existingWelcome) {
        existingWelcome.remove();
      }
    }

    pathInput.value = this.config?.installation_path || "";
    this.updateVerifyRepairVisibility(this.installationDetection);

    if (this.hasVerifiedInstallation()) {
      versionOverrideGroup.style.display = "block";
      versionOverrideCheckbox.checked = false;
    } else {
      versionOverrideGroup.style.display = "none";
    }

    modal.style.display = "flex";

    if (this.config?.installation_path) {
      this.validatePath(this.config.installation_path);
    }

    const canOpenDataFolders = this.hasVerifiedInstallation();
    const settingsOpenCustomBtn = document.getElementById("settings-open-custom-btn") as HTMLButtonElement;
    const settingsOpenSaveBtn = document.getElementById("settings-open-save-btn") as HTMLButtonElement;
    settingsOpenCustomBtn.hidden = !canOpenDataFolders;
    settingsOpenSaveBtn.hidden = !canOpenDataFolders;
    settingsOpenCustomBtn.disabled = !canOpenDataFolders;
    settingsOpenSaveBtn.disabled = !canOpenDataFolders;
    this.updateVerifyRepairVisibility(this.installationDetection);
    this.renderIcons();
  }

  private async validatePath(path: string) {
    const validation = document.getElementById("path-validation")!;
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;

    if (!path.trim()) {
      validation.innerHTML = "";
      validation.className = "validation-message";
      saveBtn.disabled = true;
      this.updateVerifyRepairVisibility(null);
      return;
    }

    validation.innerHTML = `<span class="validation-loading">${this.icon("loader-circle")}<span>Validating path...</span></span>`;
    validation.className = "validation-message loading";
    saveBtn.disabled = true;
    this.updateVerifyRepairVisibility(null);
    this.renderIcons();

    try {
      const detection = await invoke<InstallationDetection>("detect_installation", {
        path: path,
        exeName: this.config?.exe_path || "Makeplace.exe",
      });

      // Validate with detailed error information
      try {
        await invoke<string>("validate_path_detailed", {
          path: path,
          exeName: this.config?.exe_path || "Makeplace.exe",
          mode: detection.mode,
        });

        // Path is valid
        const statusText = this.getDetectionLabel(detection);
        const details = detection.details.length > 0 ? detection.details.map((detail) => this.escapeHtml(detail)).join("<br>") : this.escapeHtml(detection.message);
        validation.innerHTML = `
          <div class="${this.getValidationBoxClass(detection)}">
            <span class="validation-icon">${this.icon(this.getDetectionIcon(detection))}</span>
            <div class="validation-content">
              <div class="validation-main">${this.escapeHtml(statusText)}</div>
              <div class="validation-sub">${details}</div>
            </div>
          </div>
        `;
        validation.className = `validation-message valid ${detection.status}`;
        saveBtn.disabled = false;
        this.updateVerifyRepairVisibility(detection);
        this.renderIcons();
      } catch (errorInfo: any) {
        // Path validation failed with detailed error
        this.showValidationError(validation, errorInfo);
        this.updateVerifyRepairVisibility(detection);
        saveBtn.disabled = true;
      }
    } catch (error) {
      // Fallback for unexpected errors
      validation.innerHTML = `
        <div class="validation-error">
          <span class="validation-icon">${this.icon("circle-x")}</span>
          <div class="validation-content">
            <div class="validation-main">Error validating path</div>
            <div class="validation-sub">Please try again or select a different path</div>
          </div>
        </div>
      `;
      validation.className = "validation-message invalid";
      saveBtn.disabled = true;
      this.updateVerifyRepairVisibility(null);
      this.renderIcons();
    }
  }

  private showValidationError(validation: HTMLElement, errorInfo: ErrorInfo) {
    validation.innerHTML = `
      <div class="validation-error">
        <span class="validation-icon">${this.icon("circle-x")}</span>
        <div class="validation-content">
          <div class="validation-main">${this.escapeHtml(errorInfo.user_message)}</div>
          <div class="validation-sub">${this.escapeHtml(errorInfo.recovery_suggestion)}</div>
          ${errorInfo.category === ErrorCategory.Permission ? `<div class="validation-tip">${this.icon("info")}<span>Try running as administrator</span></div>` : ""}
        </div>
      </div>
    `;
    validation.className = "validation-message invalid";
    this.renderIcons();
  }

  private getValidationBoxClass(detection: InstallationDetection): string {
    return detection.status === "existing_incomplete" ? "validation-warning" : "validation-success";
  }

  private getDetectionIcon(detection: InstallationDetection): string {
    switch (detection.status) {
      case "existing_valid":
        return "check-circle-2";
      case "existing_incomplete":
        return "circle-alert";
      case "fresh_empty":
        return "folder-open";
      case "invalid_path":
      default:
        return "circle-x";
    }
  }

  private getDetectionLabel(detection: InstallationDetection): string {
    switch (detection.status) {
      case "existing_valid":
        return "Existing installation detected";
      case "existing_incomplete":
        return "Existing installation needs repair";
      case "fresh_empty":
        return "Empty folder ready for fresh install";
      case "invalid_path":
      default:
        return "Invalid installation folder";
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
      const detection = await invoke<InstallationDetection>("detect_installation", {
        path: path,
        exeName: this.config.exe_path,
      });

      if (detection.status === "invalid_path") {
        this.setStatus(AppState.ERROR, detection.message);
        return;
      }

      // Check if user is switching from an existing installation to a fresh install location
      const wasExistingInstall = this.config.installation_path && this.config.installation_mode === "update";
      const willBeFreshInstall = detection.status === "fresh_empty";

      if (wasExistingInstall && willBeFreshInstall) {
        const confirmed = await this.showConfirmation("Fresh Installation", `The selected folder doesn't contain an existing ReMakeplace installation.\n\nDo you want to perform a fresh installation at:\n${path}`);

        if (!confirmed) {
          return;
        }
      }

      this.installationDetection = detection;
      this.config.installation_path = detection.normalized_path || path;
      this.config.installation_mode = detection.mode;
      await invoke("save_config", { config: this.config });

      const modal = document.getElementById("settings-modal")!;
      modal.style.display = "none";

      this.updateUI();

      this.isFirstRun = false;
      await this.checkForUpdates();
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

  private async openGameDataFolder(folder: "custom" | "save") {
    if (!this.config?.installation_path) {
      this.showSettings();
      return;
    }

    if (!this.hasVerifiedInstallation()) {
      this.setStatus(AppState.ERROR, "Open Custom and Open Save are available only after a valid ReMakeplace installation is detected.");
      return;
    }

    try {
      await invoke("open_game_data_folder", { config: this.config, folder });
    } catch (error) {
      this.setStatus(AppState.ERROR, `Failed to open ${folder === "custom" ? "Custom" : "Save"} folder: ${error}`);
    }
  }

  private setStatus(state: AppState, message: string, error?: string) {
    this.currentStatus = { state, message, error };
    this.statusMessage.textContent = message;

    if (state === AppState.ERROR && this.progressStage === "complete") {
      this.progressSection.style.display = "none";
    }

    // Update status message styling based on state
    this.statusMessage.className = `status-message ${state}`;

    if (state === AppState.ERROR) {
      this.statusMessage.classList.add("error");
    }

    this.updateStatusVisibility();
  }

  private handleDownloadError(errorInfo: ErrorInfo) {
    this.setStatus(AppState.ERROR, errorInfo.user_message);
    this.setProgressStage("error", "Download stopped", errorInfo.recovery_suggestion || errorInfo.user_message);

    // Show detailed error in console for debugging
    console.error("Download error details:", errorInfo);

    this.setActionsBusy(false);
    this.updateButton.classList.remove("is-busy");
    this.updateButton.hidden = false;
    this.updateButton.disabled = false;
    this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
  }

  private handleErrorInfo(errorInfo: ErrorInfo) {
    this.setStatus(AppState.ERROR, errorInfo.user_message);
    this.setProgressStage("error", "Update stopped", errorInfo.recovery_suggestion || errorInfo.user_message);
    this.setActionsBusy(false);
    this.updateButton.classList.remove("is-busy");
    this.updateButton.hidden = false;
    this.updateButton.disabled = false;
    this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
    console.error("Error details:", errorInfo);
  }

  private handleLegacyError(errorMsg: string) {
    let userFriendlyMsg = "An error occurred";

    if (errorMsg.includes("Extraction failed")) {
      userFriendlyMsg = "The downloaded release could not be installed. Try clearing the cache and downloading again. If it keeps happening, the release archive may need to be checked.";
    } else if (errorMsg.includes("Backup failed")) {
      userFriendlyMsg = "Failed to backup your data before updating. Check that you have sufficient disk space.";
    } else if (errorMsg.includes("Failed to restore")) {
      userFriendlyMsg = "Update completed but failed to restore some user data. Check your installation directory.";
    }

    console.error("Update error details:", errorMsg);
    this.setStatus(AppState.ERROR, userFriendlyMsg);
    this.setProgressStage("error", "Update stopped", userFriendlyMsg);
    this.setActionsBusy(false);
    this.updateButton.classList.remove("is-busy");
    this.updateButton.hidden = false;
    this.updateButton.disabled = false;
    this.setButtonContent(this.updateButton, "refresh-cw", "Retry");
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
