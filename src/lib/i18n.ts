import zhCN from '../i18n/zh-CN/common.json';
import enUS from '../i18n/en-US/common.json';

type Locale = 'zh-CN' | 'en-US';

type NestedRecord = { [key: string]: string | NestedRecord };

const locales: Record<Locale, NestedRecord> = {
  'zh-CN': zhCN as unknown as NestedRecord,
  'en-US': enUS as unknown as NestedRecord,
};

let currentLocale: Locale = 'zh-CN';
const listeners: Set<() => void> = new Set();

/**
 * Get the current locale.
 */
export function getLocale(): Locale {
  return currentLocale;
}

/**
 * Set the active locale. Notifies all listeners immediately.
 */
export function setLocale(locale: Locale): void {
  if (locale !== currentLocale && locales[locale]) {
    currentLocale = locale;
    listeners.forEach((fn) => fn());
  }
}

/**
 * Subscribe to locale changes. Returns an unsubscribe function.
 */
export function onLocaleChange(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/**
 * Get a translated string by dot-notation key.
 * Falls back to zh-CN if key is missing in the active locale.
 * Returns the key itself if not found in any locale.
 *
 * @example t('home.parseButton') => "解析并下载"
 */
export function t(key: string): string {
  const value = getNestedValue(locales[currentLocale], key);
  if (typeof value === 'string') return value;

  // Fallback to zh-CN
  if (currentLocale !== 'zh-CN') {
    const fallback = getNestedValue(locales['zh-CN'], key);
    if (typeof fallback === 'string') {
      if (import.meta.env.DEV) {
        console.warn(`[i18n] Missing key "${key}" in locale "${currentLocale}", using zh-CN fallback`);
      }
      return fallback;
    }
  }

  if (import.meta.env.DEV) {
    console.warn(`[i18n] Missing key "${key}" in all locales`);
  }
  return key;
}

function getNestedValue(obj: NestedRecord | undefined, path: string): string | NestedRecord | undefined {
  if (!obj) return undefined;
  const keys = path.split('.');
  let current: string | NestedRecord | undefined = obj;
  for (const k of keys) {
    if (typeof current !== 'object' || current === null) return undefined;
    current = (current as NestedRecord)[k];
  }
  return current;
}

export type { Locale };
