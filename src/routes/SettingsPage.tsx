import { useEffect } from 'react';
import { Folder } from 'lucide-react';
import { t } from '../lib/i18n';
import { getSettings, updateSettings, selectDirectory } from '../lib/tauri';
import { setLocale } from '../lib/i18n';
import { useStore } from '../hooks/useStore';
import { useTheme } from '../hooks/useTheme';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import type { AppSettings } from '../lib/types';

export function SettingsPage() {
  const { settings, setSettings } = useStore();
  const { setTheme } = useTheme();

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const s = await getSettings();
      setSettings(s);
    } catch {
      // Use defaults
    }
  };

  const handleUpdate = async (updates: Partial<AppSettings>) => {
    const newSettings = { ...settings, ...updates };
    setSettings(newSettings);

    try {
      await updateSettings(newSettings);
    } catch {
      // Revert on failure
      loadSettings();
    }
  };

  const handleSelectDir = async () => {
    try {
      const dir = await selectDirectory();
      if (dir) {
        handleUpdate({ download_dir: dir });
      }
    } catch {
      // Dialog cancelled
    }
  };

  const handleThemeChange = (theme: 'system' | 'light' | 'dark') => {
    setTheme(theme);
    handleUpdate({ theme });
  };

  const handleLanguageChange = (language: 'zh-CN' | 'en-US') => {
    setLocale(language);
    handleUpdate({ language });
  };

  return (
    <div className="flex flex-col gap-6 p-4 md:p-6 max-w-2xl mx-auto">
      <h1 className="text-xl font-bold text-gray-900 dark:text-white">
        {t('settings.title')}
      </h1>

      {/* Download Directory */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {t('settings.downloadDir')}
        </label>
        <div className="flex gap-2">
          <Input
            value={settings.download_dir}
            readOnly
            className="flex-1"
            aria-label={t('settings.downloadDir')}
          />
          <Button variant="secondary" onClick={handleSelectDir} aria-label={t('settings.downloadDirSelect')}>
            <Folder className="w-4 h-4 mr-1" />
            {t('settings.downloadDirSelect')}
          </Button>
        </div>
      </div>

      {/* Max Concurrency */}
      <div className="space-y-2">
        <label htmlFor="max-concurrency" className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {t('settings.maxConcurrency')}
        </label>
        <input
          id="max-concurrency"
          type="range"
          min={1}
          max={8}
          value={settings.max_concurrency}
          onChange={(e) => handleUpdate({ max_concurrency: Number(e.target.value) })}
          className="w-full"
        />
        <span className="text-sm text-gray-500 dark:text-gray-400">{settings.max_concurrency}</span>
      </div>

      {/* Filename Template */}
      <div className="space-y-2">
        <label htmlFor="filename-template" className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {t('settings.filenameTemplate')}
        </label>
        <Input
          id="filename-template"
          value={settings.filename_template}
          onChange={(e) => handleUpdate({ filename_template: e.target.value })}
          aria-label={t('settings.filenameTemplate')}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500">
          {t('settings.filenameTemplateHint')}
        </p>
      </div>

      {/* Toggles */}
      <div className="space-y-4">
        <ToggleSetting
          id="auto-clipboard"
          label={t('settings.autoClipboard')}
          hint={t('settings.autoClipboardHint')}
          checked={settings.auto_clipboard}
          onChange={(v) => handleUpdate({ auto_clipboard: v })}
        />
        <ToggleSetting
          id="keep-history"
          label={t('settings.keepHistory')}
          checked={settings.keep_history}
          onChange={(v) => handleUpdate({ keep_history: v })}
        />
        <ToggleSetting
          id="debug-log"
          label={t('settings.debugLog')}
          hint={t('settings.debugLogHint')}
          checked={settings.debug_log}
          onChange={(v) => handleUpdate({ debug_log: v })}
        />
      </div>

      {/* Theme */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {t('settings.theme')}
        </label>
        <div className="flex gap-2">
          {(['system', 'light', 'dark'] as const).map((theme) => (
            <button
              key={theme}
              onClick={() => handleThemeChange(theme)}
              className={`px-4 py-2 text-sm rounded-lg transition-colors ${
                settings.theme === theme
                  ? 'bg-blue-500 text-white'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300'
              }`}
              aria-label={t(`settings.theme${theme.charAt(0).toUpperCase() + theme.slice(1)}`)}
            >
              {t(`settings.theme${theme.charAt(0).toUpperCase() + theme.slice(1)}`)}
            </button>
          ))}
        </div>
      </div>

      {/* Language */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {t('settings.language')}
        </label>
        <div className="flex gap-2">
          {([['zh-CN', '中文'], ['en-US', 'English']] as const).map(([code, label]) => (
            <button
              key={code}
              onClick={() => handleLanguageChange(code)}
              className={`px-4 py-2 text-sm rounded-lg transition-colors ${
                settings.language === code
                  ? 'bg-blue-500 text-white'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300'
              }`}
              aria-label={label}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

function ToggleSetting({
  id,
  label,
  hint,
  checked,
  onChange,
}: {
  id: string;
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <label htmlFor={id} className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {label}
        </label>
        {hint && <p className="text-xs text-gray-400 dark:text-gray-500">{hint}</p>}
      </div>
      <button
        id={id}
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative w-11 h-6 rounded-full transition-colors ${
          checked ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${
            checked ? 'translate-x-5' : ''
          }`}
        />
      </button>
    </div>
  );
}
