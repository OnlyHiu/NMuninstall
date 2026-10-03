import { describe, expect, it } from 'vitest';
import { nextSort, sortPrograms } from './sort';
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
  program({ id: 'b', displayName: 'beta', displayVersion: '2.0', estimatedSize: 5000, publisher: 'Zeta' }),
  program({ id: 'a', displayName: 'Alpha', displayVersion: '10.0', estimatedSize: 1000, publisher: 'Acme' }),
  program({ id: 'c', displayName: 'gamma' }),
  program({ id: 'd', displayName: 'Delta', displayVersion: '1.0', publisher: 'Acme' }),
];

describe('sortPrograms', () => {
  it('is case-insensitive on names', () => {
    const out = sortPrograms(data, 'displayName', 'asc');
    expect(out.map((p) => p.displayName)).toEqual(['Alpha', 'beta', 'Delta', 'gamma']);
  });

  it('reverses for desc', () => {
    const out = sortPrograms(data, 'displayName', 'desc');
    expect(out.map((p) => p.displayName)).toEqual(['gamma', 'Delta', 'beta', 'Alpha']);
  });

  it('keeps missing values last in both directions', () => {
    const asc = sortPrograms(data, 'displayVersion', 'asc');
    const desc = sortPrograms(data, 'displayVersion', 'desc');
    expect(asc[asc.length - 1].displayName).toBe('gamma');
    expect(desc[desc.length - 1].displayName).toBe('gamma');
  });

  it('sorts sizes numerically, not lexicographically', () => {
    const out = sortPrograms(data, 'estimatedSize', 'asc');
    expect(out.map((p) => p.estimatedSize)).toEqual([1000, 5000, undefined, undefined]);
  });

  it('never mutates the input', () => {
    const input = [...data];
    const copy = [...input];
    sortPrograms(input, 'displayName', 'desc');
    expect(input).toEqual(copy);
  });

  it('is deterministic for equal keys', () => {
    const dupes = [
      program({ id: 'x', displayName: 'Same', displayVersion: '1' }),
      program({ id: 'y', displayName: 'Same', displayVersion: '1' }),
    ];
    const a = sortPrograms(dupes, 'displayName', 'asc').map((p) => p.id);
    const b = sortPrograms([...dupes].reverse(), 'displayName', 'asc').map((p) => p.id);
    expect(a).toEqual(b);
  });
});

describe('nextSort', () => {
  it('starts ascending for text columns', () => {
    expect(nextSort({ field: 'displayName', order: 'asc' }, 'publisher')).toEqual({
      field: 'publisher',
      order: 'asc',
    });
  });

  it('starts descending for size', () => {
    expect(nextSort({ field: 'displayName', order: 'asc' }, 'estimatedSize')).toEqual({
      field: 'estimatedSize',
      order: 'desc',
    });
  });

  it('toggles when the same column is clicked again', () => {
    expect(nextSort({ field: 'publisher', order: 'asc' }, 'publisher')).toEqual({
      field: 'publisher',
      order: 'desc',
    });
    expect(nextSort({ field: 'publisher', order: 'desc' }, 'publisher')).toEqual({
      field: 'publisher',
      order: 'asc',
    });
  });
});
