import { describe, expect, it } from 'vitest';
import { formatDate, formatDuration, formatSize, orDash } from './format';

describe('formatSize', () => {
  it('renders an em dash for unknown or zero sizes', () => {
    expect(formatSize(undefined)).toBe('—');
    expect(formatSize(null)).toBe('—');
    expect(formatSize(0)).toBe('—');
    expect(formatSize(-5)).toBe('—');
    expect(formatSize(Number.NaN)).toBe('—');
  });

  it('keeps raw bytes without decimals', () => {
    expect(formatSize(1)).toBe('1 B');
    expect(formatSize(1023)).toBe('1023 B');
  });

  it('uses one decimal below 100 and none above', () => {
    expect(formatSize(1024)).toBe('1.0 KB');
    expect(formatSize(1024 * 100)).toBe('100 KB');
    expect(formatSize(1024 * 1024)).toBe('1.0 MB');
    expect(formatSize(1024 * 1024 * 1024)).toBe('1.0 GB');
    expect(formatSize(1024 ** 4)).toBe('1.0 TB');
  });

  it('caps at the largest unit', () => {
    // 1024 PB stays in PB rather than inventing an EB unit.
    expect(formatSize(1024 ** 6)).toBe('1024 PB');
  });
});

describe('formatDate', () => {
  it('em dash for missing values', () => {
    expect(formatDate(undefined, 'zh-CN')).toBe('—');
    expect(formatDate('', 'zh-CN')).toBe('—');
  });

  it('formats ISO dates per language', () => {
    expect(formatDate('2024-01-15', 'zh-CN')).toBe('2024/01/15');
    expect(formatDate('2024-01-15', 'en-US')).toBe('01/15/2024');
  });

  it('passes through values it cannot parse', () => {
    expect(formatDate('2024AB15', 'zh-CN')).toBe('2024AB15');
    expect(formatDate('01/15/2024', 'zh-CN')).toBe('01/15/2024');
  });
});

describe('orDash', () => {
  it('trims and collapses blanks', () => {
    expect(orDash('  x  ')).toBe('x');
    expect(orDash('   ')).toBe('—');
    expect(orDash(undefined)).toBe('—');
  });
});

describe('formatDuration', () => {
  it('uses ms below one second and seconds above', () => {
    expect(formatDuration(340)).toBe('340ms');
    expect(formatDuration(1200)).toBe('1.2s');
    expect(formatDuration(0)).toBe('0ms');
    expect(formatDuration(undefined)).toBe('—');
    expect(formatDuration(-1)).toBe('—');
  });
});
