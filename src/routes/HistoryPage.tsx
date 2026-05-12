import { useEffect, useState } from 'react';
import { Search, Trash2, Copy, CheckCircle, XCircle, Ban } from 'lucide-react';
import { t } from '../lib/i18n';
import { getHistory, clearHistory } from '../lib/tauri';
import { useStore } from '../hooks/useStore';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Badge } from '../components/ui/Badge';
import type { HistoryEntry, TaskStatus } from '../lib/types';

export function HistoryPage() {
  const { history, setHistory } = useStore();
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => {
    loadHistory();
  }, [search, statusFilter]);

  const loadHistory = async () => {
    try {
      const entries = await getHistory(search || undefined, statusFilter || undefined);
      setHistory(entries);
    } catch {
      // Handle error silently
    }
  };

  const handleClearHistory = async () => {
    try {
      await clearHistory();
      setHistory([]);
      setShowClearConfirm(false);
    } catch {
      // Handle error
    }
  };

  const handleCopyLink = async (entry: HistoryEntry) => {
    try {
      await navigator.clipboard.writeText(entry.url);
      setCopiedId(entry.id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      // Clipboard write failed
    }
  };

  const getStatusIcon = (status: TaskStatus) => {
    switch (status) {
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'failed':
        return <XCircle className="w-4 h-4 text-red-500" />;
      case 'cancelled':
        return <Ban className="w-4 h-4 text-gray-500" />;
      default:
        return null;
    }
  };

  const filters: { key: string; label: string; value: string }[] = [
    { key: 'all', label: t('history.filterAll'), value: '' },
    { key: 'completed', label: t('history.filterCompleted'), value: 'completed' },
    { key: 'failed', label: t('history.filterFailed'), value: 'failed' },
    { key: 'cancelled', label: t('history.filterCancelled'), value: 'cancelled' },
  ];

  return (
    <div className="flex flex-col gap-4 p-4 md:p-6 max-w-3xl mx-auto">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-gray-900 dark:text-white">
          {t('history.title')}
        </h1>
        <Button
          variant="destructive"
          size="sm"
          onClick={() => setShowClearConfirm(true)}
          disabled={history.length === 0}
          aria-label={t('history.clearHistory')}
        >
          <Trash2 className="w-4 h-4 mr-1" />
          {t('history.clearHistory')}
        </Button>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
        <Input
          className="pl-10"
          placeholder={t('history.searchPlaceholder')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          aria-label={t('history.searchPlaceholder')}
        />
      </div>

      {/* Filters */}
      <div className="flex gap-2 flex-wrap">
        {filters.map((f) => (
          <button
            key={f.key}
            onClick={() => setStatusFilter(f.value)}
            className={`px-3 py-1 text-sm rounded-full transition-colors ${
              statusFilter === f.value
                ? 'bg-blue-500 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
            }`}
            aria-label={f.label}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* History List */}
      {history.length === 0 ? (
        <div className="text-center py-12 text-gray-400 dark:text-gray-500">
          <p className="text-sm">{t('history.empty')}</p>
        </div>
      ) : (
        <div className="space-y-2">
          {history.map((entry) => (
            <div
              key={entry.id}
              className="p-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg flex items-center gap-3"
            >
              {getStatusIcon(entry.status)}
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                  {entry.title || entry.url}
                </p>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant="outline">{entry.platform}</Badge>
                  {entry.author && (
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      {entry.author}
                    </span>
                  )}
                </div>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleCopyLink(entry)}
                aria-label={t('history.copyLink')}
              >
                <Copy className="w-4 h-4" />
                {copiedId === entry.id && (
                  <span className="ml-1 text-xs text-green-500">✓</span>
                )}
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* Clear Confirmation Dialog */}
      {showClearConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 max-w-sm w-full shadow-xl">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
              {t('history.clearConfirmTitle')}
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-300 mb-4">
              {t('history.clearConfirm')}
            </p>
            <div className="flex gap-2 justify-end">
              <Button variant="secondary" onClick={() => setShowClearConfirm(false)}>
                {t('history.cancelButton')}
              </Button>
              <Button variant="destructive" onClick={handleClearHistory}>
                {t('history.confirmButton')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
