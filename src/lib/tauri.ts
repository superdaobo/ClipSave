import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, HistoryEntry, ResolvedMedia } from './types';

/** Parse share text/links and resolve media. */
export async function parseLinks(input: string): Promise<ResolvedMedia[]> {
  return invoke<ResolvedMedia[]>('parse_links', { input });
}

/** Add a resolved media item to the download queue. */
export async function addDownloadTask(url: string, platform: string): Promise<string> {
  return invoke<string>('add_download_task', { url, platform });
}

/** Pause a running download task. */
export async function pauseTask(id: string): Promise<void> {
  return invoke('pause_task', { id });
}

/** Resume a paused download task. */
export async function resumeTask(id: string): Promise<void> {
  return invoke('resume_task', { id });
}

/** Cancel a download task. */
export async function cancelTask(id: string): Promise<void> {
  return invoke('cancel_task', { id });
}

/** Retry a failed download task. */
export async function retryTask(id: string): Promise<void> {
  return invoke('retry_task', { id });
}

/** Open a downloaded file. */
export async function openFile(path: string): Promise<void> {
  return invoke('open_file', { path });
}

/** Open the folder containing a downloaded file. */
export async function openFolder(path: string): Promise<void> {
  return invoke('open_folder', { path });
}

/** Get current application settings. */
export async function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('get_settings');
}

/** Update application settings. */
export async function updateSettings(settings: AppSettings): Promise<void> {
  return invoke('update_settings', { settings });
}

/** Get download history with optional search and filter. */
export async function getHistory(
  search?: string,
  statusFilter?: string
): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>('get_history', {
    search: search || null,
    statusFilter: statusFilter || null,
  });
}

/** Clear all history entries. */
export async function clearHistory(): Promise<void> {
  return invoke('clear_history');
}

/** Open directory selection dialog. */
export async function selectDirectory(): Promise<string | null> {
  return invoke<string | null>('select_directory');
}

/** Read clipboard text content (explicit invocation only). */
export async function readClipboard(): Promise<string> {
  return invoke<string>('read_clipboard');
}
