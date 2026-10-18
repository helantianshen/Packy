export interface DownloadSettings {
  max_concurrent_downloads: number;
  check_timeout_seconds: number;
  download_timeout_seconds: number;
  max_attempts: number;
  speed_limit_kib: number;
  proxies: Record<string, string>;
}
export type DownloadOverrides = Partial<DownloadSettings>;
export interface TaskStatus {
  task_id: string;
  instance_id: string;
  sequence: number;
  phase: string;
  completed_files: number;
  total_files: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  bytes_per_second: number;
  error?: { code: string; message: string; request_id?: string };
}
export interface Plan {
  pack_id: string;
  pack_name: string;
  target_release_id: string;
  target_version: string;
  changelog: string;
  operations: {
    action: 'ADD' | 'REPLACE' | 'REMOVE';
    mod_id: string;
    old_path?: string;
    target?: {
      path: string;
      size: number | null;
      sha512: string;
      sources: { type: string; url: string; priority: number }[];
    };
  }[];
}
export interface ClientSnapshot {
  can_launch: boolean;
  config: {
    pack_name: string;
    pack_id: string;
    server_url: string;
    game_dir: string;
    launcher_path: string;
  };
  download: DownloadSettings;
  download_defaults: DownloadSettings;
  preferences: { background_enabled: boolean; download?: DownloadOverrides };
  state: { installed_release_id: string };
  status: TaskStatus;
  plan: Plan | null;
}
declare global {
  interface Window {
    packy?: {
      branding: () => Promise<{ logo?: string; light?: string; dark?: string }>;
      onUnavailable: (listener: (message: string) => void) => () => void;
      invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
      onStatus: (listener: (snapshot: ClientSnapshot) => void) => () => void;
    };
  }
}
