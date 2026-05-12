import { useState } from 'react';
import { Bug, Play, Trash2, Copy } from 'lucide-react';
import { parseLinks, addDownloadTask } from '../lib/tauri';
import { Button } from '../components/ui/Button';
import type { ResolvedMedia } from '../lib/types';

interface LogEntry {
  timestamp: string;
  type: 'info' | 'error' | 'success';
  message: string;
}

export function DebugPage() {
  const [inputValue, setInputValue] = useState('');
  const [parseResult, setParseResult] = useState<ResolvedMedia[] | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const addLog = (type: LogEntry['type'], message: string) => {
    setLogs((prev) => [
      ...prev,
      { timestamp: new Date().toISOString(), type, message },
    ]);
  };

  const handleParse = async () => {
    if (!inputValue.trim()) return;

    setIsLoading(true);
    setParseResult(null);
    addLog('info', `Parsing: ${inputValue.trim()}`);

    try {
      const results = await parseLinks(inputValue);
      setParseResult(results);
      addLog('success', `Parse returned ${results.length} result(s) with ${results.reduce((sum, r) => sum + r.media_items.length, 0)} media item(s)`);
    } catch (err) {
      const errorStr = JSON.stringify(err, null, 2);
      addLog('error', `Parse failed: ${errorStr}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleDownload = async (url: string, platform: string) => {
    addLog('info', `Adding download task: ${url.substring(0, 80)}...`);
    try {
      const taskId = await addDownloadTask(url, platform);
      addLog('success', `Task created: ${taskId}`);
    } catch (err) {
      addLog('error', `Download failed: ${JSON.stringify(err)}`);
    }
  };

  const handleClearLogs = () => {
    setLogs([]);
  };

  const handleCopyResult = () => {
    if (parseResult) {
      navigator.clipboard.writeText(JSON.stringify(parseResult, null, 2));
      addLog('info', 'Result copied to clipboard');
    }
  };

  return (
    <div className="flex flex-col gap-4 p-4 md:p-6 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center gap-2">
        <Bug className="w-5 h-5 text-orange-500" />
        <h1 className="text-xl font-bold text-gray-900 dark:text-white">
          Debug / Test
        </h1>
      </div>

      {/* Input */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          URL Input
        </label>
        <textarea
          className="w-full min-h-[80px] p-3 border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-400 font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-orange-500"
          placeholder="Paste a Douyin/Xiaohongshu URL here..."
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
        />
        <div className="flex gap-2">
          <Button
            onClick={handleParse}
            disabled={isLoading || !inputValue.trim()}
          >
            <Play className="w-4 h-4 mr-1" />
            {isLoading ? 'Parsing...' : 'Parse'}
          </Button>
        </div>
      </div>

      {/* Parse Result */}
      {parseResult && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Parse Result (JSON)
            </h2>
            <Button variant="secondary" onClick={handleCopyResult}>
              <Copy className="w-3 h-3 mr-1" />
              Copy
            </Button>
          </div>
          <pre className="p-3 bg-gray-900 text-green-400 rounded-lg text-xs overflow-auto max-h-[400px] font-mono">
            {JSON.stringify(parseResult, null, 2)}
          </pre>

          {/* Quick download buttons */}
          {parseResult.map((media, idx) => (
            <div key={idx} className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg space-y-2">
              <p className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {media.platform} — {media.title || 'Untitled'} ({media.media_items.length} items)
              </p>
              <div className="flex flex-wrap gap-2">
                {media.media_items.map((item, itemIdx) => (
                  <Button
                    key={itemIdx}
                    variant="secondary"
                    onClick={() => handleDownload(item.url, media.platform)}
                  >
                    Download #{itemIdx + 1}
                    {item.quality_label && ` (${item.quality_label})`}
                  </Button>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Logs */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Logs ({logs.length})
          </h2>
          <Button variant="secondary" onClick={handleClearLogs}>
            <Trash2 className="w-3 h-3 mr-1" />
            Clear
          </Button>
        </div>
        <div className="p-3 bg-gray-900 rounded-lg max-h-[300px] overflow-auto font-mono text-xs space-y-1">
          {logs.length === 0 ? (
            <p className="text-gray-500">No logs yet. Parse a URL to see activity.</p>
          ) : (
            logs.map((log, idx) => (
              <div
                key={idx}
                className={`${
                  log.type === 'error'
                    ? 'text-red-400'
                    : log.type === 'success'
                    ? 'text-green-400'
                    : 'text-gray-300'
                }`}
              >
                <span className="text-gray-500">
                  [{new Date(log.timestamp).toLocaleTimeString()}]
                </span>{' '}
                {log.message}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
