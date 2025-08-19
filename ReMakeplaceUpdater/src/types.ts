export type InstallationMode = "update" | "fresh_install";

export interface Config {
  current_version: string;
  github_repo: string;
  installation_path: string;
  exe_path: string;
  preserve_folders: string[];
  update_check_url: string;
  last_check: string;
  auto_check: boolean;
  installation_mode: InstallationMode;
}

export interface UpdateInfo {
  latest_version: string;
  download_url: string;
  is_available: boolean;
}

export interface ProgressInfo {
  percentage: number;
  speed: number; // MB/s
  downloaded: number;
  total: number;
  retry_count: number;
  is_retrying: boolean;
  retry_reason?: string;
}

export enum AppState {
  IDLE = "idle",
  CHECKING_UPDATES = "checking_updates",
  UPDATE_AVAILABLE = "update_available",
  DOWNLOADING = "downloading",
  INSTALLING = "installing",
  UP_TO_DATE = "up_to_date",
  ERROR = "error",
  FRESH_INSTALL_READY = "fresh_install_ready",
  NO_INSTALLATION = "no_installation",
}

export interface AppStatus {
  state: AppState;
  message: string;
  error?: string;
}

export interface ErrorInfo {
  category: ErrorCategory;
  user_message: string;
  technical_details: string;
  recovery_suggestion: string;
  is_retryable: boolean;
}

export enum ErrorCategory {
  Network = "Network",
  FileSystem = "FileSystem", 
  Permission = "Permission",
  Validation = "Validation",
  Configuration = "Configuration",
  Archive = "Archive",
  Unknown = "Unknown",
}

export interface ValidationResult {
  is_valid: boolean;
  error_info?: ErrorInfo;
  mode_description?: string;
}
