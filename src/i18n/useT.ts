import { useMemo } from 'react';
import type { Language } from '@/types';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { makeT, type TFn } from './index';

/**
 * Translator bound to the current language setting.
 *
 * Lives in its own module so components do not need to know how the language
 * is resolved, and so the memoised function identity stays stable per language
 * (important for `useMemo` dependencies).
 */
export function useT(): TFn {
  const lang = useSettingsStore((s) => s.settings.language) as Language;
  return useMemo(() => makeT(lang), [lang]);
}

/** Current language, for `Intl`-aware formatting. */
export function useLang(): Language {
  return useSettingsStore((s) => s.settings.language) as Language;
}
