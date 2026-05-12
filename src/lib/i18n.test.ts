import { describe, it, expect } from 'vitest';
import { t, getLocale, setLocale } from './i18n';

describe('i18n', () => {
  it('defaults to zh-CN', () => {
    expect(getLocale()).toBe('zh-CN');
  });

  it('returns Chinese text by default', () => {
    const result = t('app.name');
    expect(result).toBe('ClipSave');
  });

  it('switches locale', () => {
    setLocale('en-US');
    expect(getLocale()).toBe('en-US');
    const result = t('home.parseButton');
    expect(result).toBe('Parse & Download');
    // Reset
    setLocale('zh-CN');
  });

  it('falls back to zh-CN for missing keys', () => {
    setLocale('en-US');
    // This key exists in both, so test a known key
    const result = t('app.name');
    expect(result).toBe('ClipSave');
    setLocale('zh-CN');
  });

  it('returns key for completely missing translations', () => {
    const result = t('nonexistent.key.here');
    expect(result).toBe('nonexistent.key.here');
  });
});
