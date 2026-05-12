import { t } from './i18n';
import type { AppError } from './types';

/**
 * Map a backend AppError to a user-friendly localized message.
 * Never exposes raw panic messages, backtraces, or internal identifiers.
 */
export function getErrorMessage(error: AppError): string {
  const code = error.code;
  const i18nKey = `errors.${code}`;
  const message = t(i18nKey);

  // If the i18n key resolves to itself, use a generic message
  if (message === i18nKey) {
    return t('errors.NetworkError');
  }

  return message;
}

/**
 * Check if an error is of type RestrictedContent (should not offer retry).
 */
export function isRestrictedContent(error: AppError): boolean {
  return error.code === 'RestrictedContent';
}

/**
 * Create a redacted error report for copying.
 * Includes: category, code, timestamp, URL host only.
 * Does NOT include: full URL, cookies, tokens, credentials, absolute paths with user home.
 */
export function createRedactedReport(error: AppError, urlHost?: string): string {
  const timestamp = new Date().toISOString();
  const lines = [
    `Error Code: ${error.code}`,
    `Timestamp: ${timestamp}`,
  ];

  if (urlHost) {
    lines.push(`Host: ${urlHost}`);
  }

  if (error.details?.message) {
    // Redact any path-like content that might contain user home
    const safeMessage = error.details.message
      .replace(/[A-Z]:\\Users\\[^\\]+/gi, '[REDACTED_PATH]')
      .replace(/\/home\/[^/]+/g, '[REDACTED_PATH]')
      .replace(/\/Users\/[^/]+/g, '[REDACTED_PATH]');
    lines.push(`Message: ${safeMessage}`);
  }

  return lines.join('\n');
}
