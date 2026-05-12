import {
  Pause,
  Play,
  X,
  RotateCcw,
  FolderOpen,
  FileText,
  Video,
  Image,
  Film,
} from 'lucide-react';
import { t } from '../../lib/i18n';
import { pauseTask, resumeTask, cancelTask, retryTask, openFile, openFolder } from '../../lib/tauri';
import { useStore } from '../../hooks/useStore';
import { Badge } from '../../components/ui/Badge';
import { Progress } from '../../components/ui/Progress';
import { Button } from '../../components/ui/Button';
import type { DownloadTask, TaskStatus } from '../../lib/types';

interface TaskCardProps {
  task: DownloadTask;
}

export function TaskCard({ task }: TaskCardProps) {
  const { updateTask } = useStore();

  const handlePause = async () => {
    try {
      await pauseTask(task.id);
      updateTask(task.id, { status: 'paused' });
    } catch { /* handled by event */ }
  };

  const handleResume = async () => {
    try {
      await resumeTask(task.id);
      updateTask(task.id, { status: 'downloading' });
    } catch { /* handled by event */ }
  };

  const handleCancel = async () => {
    try {
      await cancelTask(task.id);
      updateTask(task.id, { status: 'cancelled' });
    } catch { /* handled by event */ }
  };

  const handleRetry = async () => {
    try {
      await retryTask(task.id);
      updateTask(task.id, { status: 'waiting', error: null, progress: 0 });
    } catch { /* handled by event */ }
  };

  const handleOpenFile = async () => {
    if (task.save_path) {
      try { await openFile(task.save_path); } catch { /* ignore */ }
    }
  };

  const handleOpenFolder = async () => {
    if (task.save_path) {
      try { await openFolder(task.save_path); } catch { /* ignore */ }
    }
  };

  const getMediaIcon = () => {
    switch (task.media_type) {
      case 'video': return <Video className="w-4 h-4" />;
      case 'image': return <Image className="w-4 h-4" />;
      case 'gif': return <Film className="w-4 h-4" />;
      default: return <FileText className="w-4 h-4" />;
    }
  };

  const getStatusColor = (status: TaskStatus): string => {
    switch (status) {
      case 'waiting': return 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300';
      case 'parsing': return 'bg-blue-100 text-blue-600 dark:bg-blue-900 dark:text-blue-300';
      case 'downloading': return 'bg-blue-100 text-blue-600 dark:bg-blue-900 dark:text-blue-300';
      case 'paused': return 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900 dark:text-yellow-300';
      case 'completed': return 'bg-green-100 text-green-600 dark:bg-green-900 dark:text-green-300';
      case 'failed': return 'bg-red-100 text-red-600 dark:bg-red-900 dark:text-red-300';
      case 'cancelled': return 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400';
    }
  };

  const formatSpeed = (bytesPerSec: number): string => {
    if (bytesPerSec < 1024) return `${bytesPerSec} B/s`;
    if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
  };

  const canPause = task.status === 'downloading';
  const canResume = task.status === 'paused';
  const canCancel = ['waiting', 'parsing', 'downloading', 'paused'].includes(task.status);
  const canRetry = task.status === 'failed';
  const canOpen = task.status === 'completed' && task.save_path;

  return (
    <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-sm space-y-3">
      {/* Header */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2 flex-1 min-w-0">
          {getMediaIcon()}
          <span className="text-sm font-medium text-gray-900 dark:text-white truncate">
            {task.title || task.url}
          </span>
        </div>
        <Badge className={getStatusColor(task.status)}>
          {t(`task.status.${task.status}`)}
        </Badge>
      </div>

      {/* Meta */}
      <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
        <Badge variant="outline">{task.platform}</Badge>
        <Badge variant="outline">{t(`task.mediaType.${task.media_type}`)}</Badge>
        {task.author && <span>{task.author}</span>}
      </div>

      {/* Progress */}
      {(task.status === 'downloading' || task.status === 'paused') && (
        <div className="space-y-1">
          <Progress value={task.progress} aria-label={`${task.progress.toFixed(0)}%`} />
          <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
            <span>{task.progress.toFixed(1)}%</span>
            {task.status === 'downloading' && <span>{formatSpeed(task.speed)}</span>}
          </div>
        </div>
      )}

      {/* Error */}
      {task.error && (
        <p className="text-xs text-red-500 dark:text-red-400">{task.error}</p>
      )}

      {/* Actions */}
      <div className="flex gap-1 flex-wrap">
        {canPause && (
          <Button size="sm" variant="ghost" onClick={handlePause} aria-label={t('task.actions.pause')}>
            <Pause className="w-3 h-3 mr-1" /> {t('task.actions.pause')}
          </Button>
        )}
        {canResume && (
          <Button size="sm" variant="ghost" onClick={handleResume} aria-label={t('task.actions.resume')}>
            <Play className="w-3 h-3 mr-1" /> {t('task.actions.resume')}
          </Button>
        )}
        {canCancel && (
          <Button size="sm" variant="ghost" onClick={handleCancel} aria-label={t('task.actions.cancel')}>
            <X className="w-3 h-3 mr-1" /> {t('task.actions.cancel')}
          </Button>
        )}
        {canRetry && (
          <Button size="sm" variant="ghost" onClick={handleRetry} aria-label={t('task.actions.retry')}>
            <RotateCcw className="w-3 h-3 mr-1" /> {t('task.actions.retry')}
          </Button>
        )}
        {canOpen && (
          <>
            <Button size="sm" variant="ghost" onClick={handleOpenFile} aria-label={t('task.actions.openFile')}>
              <FileText className="w-3 h-3 mr-1" /> {t('task.actions.openFile')}
            </Button>
            <Button size="sm" variant="ghost" onClick={handleOpenFolder} aria-label={t('task.actions.openFolder')}>
              <FolderOpen className="w-3 h-3 mr-1" /> {t('task.actions.openFolder')}
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
