import { useState } from 'react';
import { Download, Clipboard, AlertCircle, Image, CheckSquare, Square } from 'lucide-react';
import { t } from '../lib/i18n';
import { parseLinks, addDownloadTask, readClipboard } from '../lib/tauri';
import { useStore } from '../hooks/useStore';
import { getErrorMessage } from '../lib/errors';
import { TaskCard } from '../features/downloads/TaskCard';
import { Button } from '../components/ui/Button';
import type { AppError, DownloadTask, ResolvedMedia } from '../lib/types';

/** Track which items are selected and which quality is chosen per item */
interface SelectionState {
  /** Map of "mediaIdx-itemIdx" -> selected quality URL */
  selectedItems: Record<string, string>;
  /** Map of "mediaIdx-itemIdx" -> whether checked for batch download */
  checkedItems: Record<string, boolean>;
}

export function HomePage() {
  const [inputValue, setInputValue] = useState('');
  const [parseError, setParseError] = useState<string | null>(null);
  const [parsedResults, setParsedResults] = useState<ResolvedMedia[] | null>(null);
  const [selection, setSelection] = useState<SelectionState>({
    selectedItems: {},
    checkedItems: {},
  });
  const { tasks, addTask, isLoading, setLoading } = useStore();

  const handleParse = async () => {
    if (!inputValue.trim()) return;

    setParseError(null);
    setParsedResults(null);
    setLoading(true);

    try {
      const results: ResolvedMedia[] = await parseLinks(inputValue);

      if (results.length === 0) {
        setParseError(t('home.noLinksFound'));
      } else {
        setParsedResults(results);
        // Initialize selection: check all items, select first (highest quality) URL
        const checked: Record<string, boolean> = {};
        const selected: Record<string, string> = {};
        results.forEach((media, mIdx) => {
          media.media_items.forEach((item, iIdx) => {
            const key = `${mIdx}-${iIdx}`;
            // For items with quality variants, only check the first one (best quality)
            // For multi-image posts, check all
            if (media.media_items.length > 1 && item.quality_label) {
              // Multiple quality variants of same video - only check best
              checked[key] = iIdx === 0;
            } else {
              checked[key] = true;
            }
            selected[key] = item.url;
          });
        });
        setSelection({ checkedItems: checked, selectedItems: selected });
      }
    } catch (err) {
      const appError = err as AppError;
      setParseError(getErrorMessage(appError));
    } finally {
      setLoading(false);
    }
  };

  const handleDownloadSelected = async () => {
    if (!parsedResults) return;

    setLoading(true);
    try {
      for (const [key, isChecked] of Object.entries(selection.checkedItems)) {
        if (!isChecked) continue;

        const [mIdxStr, iIdxStr] = key.split('-');
        const mIdx = parseInt(mIdxStr ?? '0');
        const iIdx = parseInt(iIdxStr ?? '0');
        const media = parsedResults[mIdx];
        if (!media) continue;
        const item = media.media_items[iIdx];
        if (!item) continue;

        const downloadUrl = selection.selectedItems[key] || item.url;
        const taskId = await addDownloadTask(downloadUrl, media.platform);
        const task: DownloadTask = {
          id: taskId,
          url: downloadUrl,
          platform: media.platform,
          status: 'waiting',
          progress: 0,
          speed: 0,
          downloaded_size: 0,
          total_size: item.size,
          save_path: null,
          error: null,
          title: media.title,
          author: media.author,
          media_type: item.media_type,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };
        addTask(task);
      }
      setParsedResults(null);
      setInputValue('');
    } catch (err) {
      const appError = err as AppError;
      setParseError(getErrorMessage(appError));
    } finally {
      setLoading(false);
    }
  };

  const handleClipboard = async () => {
    try {
      const text = await readClipboard();
      if (text) {
        setInputValue(text);
      }
    } catch {
      // Clipboard read failed silently
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleParse();
    }
  };

  const toggleCheck = (key: string) => {
    setSelection((prev) => ({
      ...prev,
      checkedItems: {
        ...prev.checkedItems,
        [key]: !prev.checkedItems[key],
      },
    }));
  };

  const toggleAll = () => {
    const allChecked = Object.values(selection.checkedItems).every(Boolean);
    const newChecked: Record<string, boolean> = {};
    for (const key of Object.keys(selection.checkedItems)) {
      newChecked[key] = !allChecked;
    }
    setSelection((prev) => ({ ...prev, checkedItems: newChecked }));
  };

  const checkedCount = Object.values(selection.checkedItems).filter(Boolean).length;
  const totalCount = Object.keys(selection.checkedItems).length;

  return (
    <div className="flex flex-col gap-6 p-4 md:p-6 max-w-3xl mx-auto">
      {/* Brand Area */}
      <div className="text-center space-y-2">
        <h1 className="text-2xl md:text-3xl font-bold text-gray-900 dark:text-white">
          {t('app.name')}
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          {t('app.subtitle')}
        </p>
      </div>

      {/* Compliance Notice */}
      <div className="flex items-center gap-2 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
        <AlertCircle className="w-4 h-4 text-amber-600 dark:text-amber-400 flex-shrink-0" />
        <p className="text-xs text-amber-700 dark:text-amber-300">
          {t('home.complianceNotice')}
        </p>
      </div>

      {/* Input Area */}
      <div className="space-y-3">
        <div className="relative">
          <textarea
            className="w-full min-h-[100px] p-4 border border-gray-200 dark:border-gray-700 rounded-xl bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            placeholder={t('home.inputPlaceholder')}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            aria-label={t('home.inputPlaceholder')}
          />
        </div>

        <div className="flex gap-2">
          <Button
            onClick={handleParse}
            disabled={isLoading || !inputValue.trim()}
            className="flex-1"
            aria-label={t('home.parseButton')}
          >
            <Download className="w-4 h-4 mr-2" />
            {t('home.parseButton')}
          </Button>
          <Button
            variant="secondary"
            onClick={handleClipboard}
            aria-label={t('home.clipboardButton')}
          >
            <Clipboard className="w-4 h-4 mr-2" />
            {t('home.clipboardButton')}
          </Button>
        </div>

        {/* Error Display */}
        {parseError && (
          <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
            <p className="text-sm text-red-700 dark:text-red-300">{parseError}</p>
          </div>
        )}
      </div>

      {/* Preview Panel - shown after parsing */}
      {parsedResults && (
        <div className="space-y-4 border border-gray-200 dark:border-gray-700 rounded-xl p-4 bg-white dark:bg-gray-800">
          {/* Batch controls */}
          <div className="flex items-center justify-between">
            <button
              onClick={toggleAll}
              className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
            >
              {checkedCount === totalCount ? (
                <CheckSquare className="w-4 h-4" />
              ) : (
                <Square className="w-4 h-4" />
              )}
              全选 / 取消全选 ({checkedCount}/{totalCount})
            </button>
            <Button
              onClick={handleDownloadSelected}
              disabled={checkedCount === 0 || isLoading}
            >
              <Download className="w-4 h-4 mr-1" />
              下载选中 ({checkedCount})
            </Button>
          </div>

          {/* Media items */}
          {parsedResults.map((media, mIdx) => (
            <div key={mIdx} className="space-y-3">
              {/* Cover preview */}
              {media.cover && (
                <div className="flex gap-3 items-start">
                  <div className="w-20 h-20 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 flex-shrink-0">
                    <img
                      src={media.cover}
                      alt={media.title || 'Cover'}
                      className="w-full h-full object-cover"
                      onError={(e) => {
                        (e.target as HTMLImageElement).style.display = 'none';
                      }}
                    />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {media.title || '未知标题'}
                    </p>
                    {media.author && (
                      <p className="text-xs text-gray-500 dark:text-gray-400">
                        @{media.author}
                      </p>
                    )}
                    <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                      {media.platform} · {media.media_items.length} 个媒体项
                    </p>
                  </div>
                </div>
              )}

              {/* If no cover, show text info */}
              {!media.cover && (
                <div className="flex items-center gap-2">
                  <Image className="w-5 h-5 text-gray-400" />
                  <div>
                    <p className="text-sm font-medium text-gray-900 dark:text-white">
                      {media.title || '未知标题'}
                    </p>
                    <p className="text-xs text-gray-500">
                      {media.platform} · {media.media_items.length} 个媒体项
                    </p>
                  </div>
                </div>
              )}

              {/* Individual media items with quality selection */}
              <div className="space-y-2 pl-2">
                {media.media_items.map((item, iIdx) => {
                  const key = `${mIdx}-${iIdx}`;
                  const isChecked = selection.checkedItems[key] ?? false;

                  return (
                    <div
                      key={iIdx}
                      className={`flex items-center gap-3 p-2 rounded-lg transition-colors ${
                        isChecked
                          ? 'bg-blue-50 dark:bg-blue-900/20'
                          : 'bg-gray-50 dark:bg-gray-700/50'
                      }`}
                    >
                      {/* Checkbox */}
                      <button
                        onClick={() => toggleCheck(key)}
                        className="flex-shrink-0"
                        aria-label={isChecked ? 'Deselect' : 'Select'}
                      >
                        {isChecked ? (
                          <CheckSquare className="w-5 h-5 text-blue-600 dark:text-blue-400" />
                        ) : (
                          <Square className="w-5 h-5 text-gray-400" />
                        )}
                      </button>

                      {/* Item info */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-xs px-1.5 py-0.5 rounded bg-gray-200 dark:bg-gray-600 text-gray-700 dark:text-gray-300">
                            {item.media_type}
                          </span>
                          {item.quality_label && (
                            <span className="text-xs px-1.5 py-0.5 rounded bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 font-medium">
                              {item.quality_label}
                            </span>
                          )}
                          {item.bitrate && (
                            <span className="text-xs text-gray-400">
                              {item.bitrate}kbps
                            </span>
                          )}
                        </div>
                        <p className="text-xs text-gray-500 dark:text-gray-400 truncate mt-0.5">
                          {item.url.substring(0, 60)}...
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>

              {/* Separator between multiple parsed results */}
              {mIdx < parsedResults.length - 1 && (
                <hr className="border-gray-200 dark:border-gray-700" />
              )}
            </div>
          ))}
        </div>
      )}

      {/* Task Queue */}
      <div className="space-y-3">
        {tasks.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">
            <Download className="w-12 h-12 mx-auto mb-3 opacity-30" />
            <p className="text-sm">{t('home.emptyQueue')}</p>
            <p className="text-xs mt-1">{t('home.emptyQueueHint')}</p>
          </div>
        ) : (
          tasks.map((task) => <TaskCard key={task.id} task={task} />)
        )}
      </div>
    </div>
  );
}
