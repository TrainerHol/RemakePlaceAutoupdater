export interface Config {
  current_version: string;
  github_repo: string;
  installation_path: string;
  exe_path: string;
  preserve_folders: string[];
  update_check_url: string;
  last_check: string;
  auto_check: boolean;
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
}

export enum AppState {
  IDLE = "idle",
  CHECKING_UPDATES = "checking_updates",
  UPDATE_AVAILABLE = "update_available",
  DOWNLOADING = "downloading",
  INSTALLING = "installing",
  UP_TO_DATE = "up_to_date",
  ERROR = "error",
}

export interface AppStatus {
  state: AppState;
  message: string;
  error?: string;
}
