import type { Language } from '@/types';

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'] as const;

/**
 * Formats a byte count the way Windows does: 1 decimal for KB and above,
 * no decimals for bytes, em dash for unknown.
 */
export function formatSize(bytes: number | undefined | null): string {
  if (bytes === undefined || bytes === null || !Number.isFinite(bytes) || bytes <= 0) {
    return '—';
  }
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

/**
 * Renders an ISO-ish `yyyy-MM-dd` string in the user's locale.
 * Anything unparseable is returned untouched so bad registry data stays visible.
 */
export function formatDate(iso: string | undefined | null, lang: Language): string {
  if (!iso) return '—';
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso.trim());
  if (!match) return iso;
  const [, y, m, d] = match;
  if (lang === 'en-US') return `${m}/${d}/${y}`;
  return `${y}/${m}/${d}`;
}

/** `—` for a missing value, otherwise trimmed text. */
export function orDash(value: string | undefined | null): string {
  const t = value?.trim();
  return t ? t : '—';
}

/** Escapes the characters that matter inside an HTML title attribute. */
export function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** Renders a millisecond duration compactly, e.g. `1.2s` or `340ms`. */
export function formatDuration(ms: number | undefined | null): string {
  if (ms === undefined || ms === null || !Number.isFinite(ms) || ms < 0) return '—';
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/** Human byte total for a residue summary. */
export function formatSizeLong(bytes: number): string {
  return formatSize(bytes);
}
