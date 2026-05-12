import { Shield, ExternalLink } from 'lucide-react';
import { t } from '../lib/i18n';

export function AboutPage() {
  return (
    <div className="flex flex-col gap-6 p-4 md:p-6 max-w-2xl mx-auto">
      <div className="text-center space-y-2">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          {t('about.title')}
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          {t('app.version')} 1.0.0
        </p>
      </div>

      <p className="text-sm text-gray-600 dark:text-gray-300 text-center">
        {t('about.description')}
      </p>

      {/* Disclaimer */}
      <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl p-4 space-y-3">
        <div className="flex items-center gap-2">
          <Shield className="w-5 h-5 text-amber-600 dark:text-amber-400" />
          <h2 className="text-sm font-semibold text-amber-800 dark:text-amber-300">
            {t('about.disclaimer')}
          </h2>
        </div>
        <ul className="space-y-2 text-sm text-amber-700 dark:text-amber-300">
          <li>• {t('about.disclaimerText1')}</li>
          <li>• {t('about.disclaimerText2')}</li>
          <li>• {t('about.disclaimerText3')}</li>
          <li>• {t('about.disclaimerText4')}</li>
        </ul>
      </div>

      {/* Open Source */}
      <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-4">
        <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-2">
          {t('about.openSource')}
        </h2>
        <a
          href="https://github.com/superdaobo/ClipSave"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1 text-sm text-blue-500 hover:text-blue-600"
        >
          GitHub
          <ExternalLink className="w-3 h-3" />
        </a>
      </div>

      {/* License */}
      <div className="text-center text-xs text-gray-400 dark:text-gray-500">
        <p>{t('about.license')}: MIT</p>
      </div>
    </div>
  );
}
