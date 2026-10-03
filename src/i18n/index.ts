import type { Language } from '@/types';
import { enUS } from './en-US';
import { zhCN, type TranslationKey } from './zh-CN';

type Dict = Record<string, string>;

const dictionaries: Record<Language, Dict> = {
  'zh-CN': zhCN as unknown as Dict,
  'en-US': enUS as unknown as Dict,
};

export type { TranslationKey };

/** Replaces `{name}` placeholders. Unknown placeholders are left as-is. */
function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (whole, key: string) =>
    key in vars ? String(vars[key]) : whole,
  );
}

const warned = new Set<string>();

/** Translates a key, falling back to zh-CN and finally to the key itself. */
export function translate(
  lang: Language,
  key: TranslationKey | string,
  vars?: Record<string, string | number>,
): string {
  const primary = dictionaries[lang];
  let text = primary[key];
  if (text === undefined) {
    text = zhCN[key as TranslationKey];
  }
  if (text === undefined) {
    if (!warned.has(key)) {
      warned.add(key);
      console.warn(`[i18n] missing translation key: ${key}`);
    }
    return key;
  }
  return interpolate(text, vars);
}

export type TFn = (key: TranslationKey | string, vars?: Record<string, string | number>) => string;

/** Builds a bound translator for a language. */
export function makeT(lang: Language): TFn {
  return (key, vars) => translate(lang, key, vars);
}
