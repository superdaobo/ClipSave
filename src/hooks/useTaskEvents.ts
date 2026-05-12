import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useStore } from './useStore';
import type { TaskProgressEvent, TaskCompletedEvent, TaskFailedEvent } from '../lib/types';

/**
 * Listen for Tauri events from the Rust backend for task progress updates.
 * Events: task-progress, task-completed, task-failed, task-status-changed
 */
export function useTaskEvents() {
  const { updateTask } = useStore();

  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    const setup = async () => {
      // Task progress updates (max every 250ms per task)
      const unlisten1 = await listen<TaskProgressEvent>('task-progress', (event) => {
        const { id, progress, speed, downloaded_size, total_size } = event.payload;
        updateTask(id, {
          progress,
          speed,
          downloaded_size,
          total_size,
          status: 'downloading',
        });
      });
      unlisteners.push(unlisten1);

      // Task completed
      const unlisten2 = await listen<TaskCompletedEvent>('task-completed', (event) => {
        const { id, save_path } = event.payload;
        updateTask(id, {
          status: 'completed',
          progress: 100,
          speed: 0,
          save_path,
        });
      });
      unlisteners.push(unlisten2);

      // Task failed
      const unlisten3 = await listen<TaskFailedEvent>('task-failed', (event) => {
        const { id, error_message } = event.payload;
        updateTask(id, {
          status: 'failed',
          speed: 0,
          error: error_message,
        });
      });
      unlisteners.push(unlisten3);

      // Task status changed (generic status update)
      const unlisten4 = await listen<{ id: string; status: string }>('task-status-changed', (event) => {
        const { id, status } = event.payload;
        updateTask(id, {
          status: status as 'downloading' | 'paused' | 'cancelled',
        });
      });
      unlisteners.push(unlisten4);
    };

    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [updateTask]);
}
