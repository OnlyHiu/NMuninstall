import { describe, expect, it } from 'vitest';
import { buildIndex, filterPrograms, parseQuery } from './search';
import { toAppError } from './invoke';
import type { ProgramInfo } from '@/types';

function program(over: Partial<ProgramInfo> & { id: string }): ProgramInfo {
  return {
    keyName: over.id,
    regPath: `HKLM\\SOFTWARE\\...\\${over.id}`,
    displayName: over.id,
    displayVersion: undefined,
    publisher: undefined,
    installDate: undefined,
    installLocation: undefined,
    estimatedSize: undefined,
    uninstallString: undefined,
    quietUninstallString: undefined,
    displayIcon: undefined,
    iconPath: undefined,
    systemComponent: false,
    windowsInstaller: false,
    parentKeyName: undefined,
    noModify: false,
    noRepair: false,
    isSystem: false,
    hive: 'HKLM',
    view: '64',
    is64Bit: true,
    canUninstall: true,
    ...over,
  };
}

const data: ProgramInfo[] = [
  program({ id: '1', displayName: 'Visual Studio Code', publisher: 'Microsoft' }),
  program({ id: '2', displayName: '7-Zip', publisher: 'Igor Pavlov', displayVersion: '23.01' }),
  program({ id: '3', displayName: 'Notepad', publisher: 'Microsoft' }),
];

describe('parseQuery', () => {
  it('lower-cases and splits on whitespace', () => {
    expect(parseQuery('  Foo   BAR ')).toEqual(['foo', 'bar']);
  });

  it('returns nothing for a blank query', () => {
    expect(parseQuery('')).toEqual([]);
    expect(parseQuery('   \t ')).toEqual([]);
  });
});

describe('buildIndex / filterPrograms', () => {
  it('returns the same array for an empty query', () => {
    expect(filterPrograms(data, '   ')).toBe(data);
  });

  it('matches on name, case-insensitively', () => {
    expect(filterPrograms(data, 'code').map((p) => p.id)).toEqual(['1']);
    expect(filterPrograms(data, 'CODE').map((p) => p.id)).toEqual(['1']);
  });

  it('matches on publisher', () => {
    expect(filterPrograms(data, 'pavlov').map((p) => p.id)).toEqual(['2']);
  });

  it('matches on version', () => {
    expect(filterPrograms(data, '23.01').map((p) => p.id)).toEqual(['2']);
  });

  it('requires every keyword to match (AND)', () => {
    expect(filterPrograms(data, 'micro').map((p) => p.id)).toEqual(['1', '3']);
    expect(filterPrograms(data, 'micro code').map((p) => p.id)).toEqual(['1']);
    expect(filterPrograms(data, 'micro nothing')).toEqual([]);
  });

  it('can reuse a prebuilt index', () => {
    const index = buildIndex(data);
    expect(index.get('1')).toContain('visual studio code');
    expect(filterPrograms(data, 'code', index).map((p) => p.id)).toEqual(['1']);
  });

  it('stays fast with 2000 rows', () => {
    const many = Array.from({ length: 2000 }, (_, i) =>
      program({ id: String(i), displayName: `Program ${i}`, publisher: `Vendor ${i % 50}` }),
    );
    const index = buildIndex(many);
    const start = performance.now();
    filterPrograms(many, 'vendor 7', index);
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(100); // F-602
  });
});

describe('toAppError', () => {
  it('passes through a serialised backend error', () => {
    const e = toAppError({ code: 'NOT_FOUND', message: '未找到该程序', detail: 'id=HKLM:64:x' });
    expect(e).toEqual({ code: 'NOT_FOUND', message: '未找到该程序', detail: 'id=HKLM:64:x' });
  });

  it('handles a plain string', () => {
    expect(toAppError('boom')).toEqual({ code: 'UNKNOWN', message: 'boom' });
  });

  it('handles an object with only a message', () => {
    expect(toAppError({ message: 'oops' })).toEqual({ code: 'UNKNOWN', message: 'oops' });
  });

  it('falls back to String() for anything else', () => {
    expect(toAppError(42).message).toBe('42');
  });
});
