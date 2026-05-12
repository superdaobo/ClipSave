import { create } from 'zustand';
import type { AppSettings, DownloadTask, HistoryEntry } from '../lib/types';

interface AppState {
  // Download tasks
  tasks: DownloadTask[];
  addTask: (task: DownloadTask) => void;
  updateTask: (id: string, updates: Partial<DownloadTask>) => void;
  removeTask: (id: string) => void;

  // History
  history: HistoryEntry[];
  setHistory: (entries: HistoryEntry[]) => void;

  // Settings
  settings: AppSettings;
  setSettings: (settings: AppSettings) => void;

  // UI state
  isLoading: boolean;
  setLoading: (loading: boolean) => void;
  error: string | null;
  setError: (error: string | null) => void;
}

export const useStore = create<AppState>((set) => ({
  // Tasks
  tasks: [],
  addTask: (task) =>
    set((state) => ({ tasks: [...state.tasks, task] })),
  updateTask: (id, updates) =>
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === id ? { ...t, ...updates } : t
      ),
    })),
  removeTask: (id) =>
    set((state) => ({
      tasks: state.tasks.filter((t) => t.id !== id),
    })),

  // History
  history: [],
  setHistory: (entries) => set({ history: entries }),

  // Settings
  settings: {
    download_dir: '',
    max_concurrency: 3,
    filename_template: '{platform}/{author}/{date}/{title}_{index}.{ext}',
    auto_clipboard: false,
    keep_history: true,
    debug_log: false,
    theme: 'system',
    language: 'zh-CN',
  },
  setSettings: (settings) => set({ settings }),

  // UI
  isLoading: false,
  setLoading: (loading) => set({ isLoading: loading }),
  error: null,
  setError: (error) => set({ error }),
}));
