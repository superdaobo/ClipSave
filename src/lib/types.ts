/** Supported platforms */
export type Platform = 'douyin' | 'xiaohongshu';

/** Media type classification */
export type MediaType = 'video' | 'image' | 'gif' | 'unknown';

/** Download task status */
export type TaskStatus =
  | 'waiting'
  | 'parsing'
  | 'downloading'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

/** A single media item within a resolved result */
export interface MediaItem {
  media_type: MediaType;
  url: string;
  filename_hint: string | null;
  mime_type: string | null;
  size: number | null;
  /** Bitrate in kbps (extracted from URL params) */
  bitrate: number | null;
  /** Human-readable quality label (e.g., "1080p", "720p") */
  quality_label: string | null;
}

/** Result of resolving a URL to its media content */
export interface ResolvedMedia {
  platform: Platform;
  source_url: string;
  canonical_url: string;
  title: string | null;
  author: string | null;
  media_items: MediaItem[];
  cover: string | null;
  created_at: string | null;
}

/** A download task entity */
export interface DownloadTask {
  id: string;
  url: string;
  platform: Platform;
  status: TaskStatus;
  progress: number;
  speed: number;
  downloaded_size: number;
  total_size: number | null;
  save_path: string | null;
  error: string | null;
  title: string | null;
  author: string | null;
  media_type: MediaType;
  created_at: string;
  updated_at: string;
}

/** Application settings */
export interface AppSettings {
  download_dir: string;
  max_concurrency: number;
  filename_template: string;
  auto_clipboard: boolean;
  keep_history: boolean;
  debug_log: boolean;
  theme: 'system' | 'light' | 'dark';
  language: 'zh-CN' | 'en-US';
}

/** History entry */
export interface HistoryEntry {
  id: string;
  url: string;
  platform: Platform;
  author: string | null;
  title: string | null;
  status: TaskStatus;
  save_path: string | null;
  created_at: string;
}

/** App error from backend */
export interface AppError {
  code: string;
  details: {
    message: string;
    platform_hint?: string;
    from?: string;
    to?: string;
  };
}

/** Task progress event payload */
export interface TaskProgressEvent {
  id: string;
  progress: number;
  speed: number;
  downloaded_size: number;
  total_size: number | null;
}

/** Task completed event payload */
export interface TaskCompletedEvent {
  id: string;
  save_path: string;
}

/** Task failed event payload */
export interface TaskFailedEvent {
  id: string;
  error_code: string;
  error_message: string;
}
