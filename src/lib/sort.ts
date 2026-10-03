import type { ProgramInfo, SortField, SortOrder } from '@/types';

/** `undefined` always sorts last, in both directions, like the Windows list views. */
function compareValues(a: unknown, b: unknown, order: SortOrder): number {
  const aMissing = a === undefined || a === null || a === '';
  const bMissing = b === undefined || b === null || b === '';
  if (aMissing && bMissing) return 0;
  if (aMissing) return 1;
  if (bMissing) return -1;

  const sign = order === 'asc' ? 1 : -1;
  if (typeof a === 'number' && typeof b === 'number') {
    return (a - b) * sign;
  }
  const as = String(a).toLowerCase();
  const bs = String(b).toLowerCase();
  if (as < bs) return -sign;
  if (as > bs) return sign;
  return 0;
}

function valueFor(p: ProgramInfo, field: SortField): unknown {
  switch (field) {
    case 'displayName':
      return p.displayName;
    case 'displayVersion':
      return p.displayVersion;
    case 'publisher':
      return p.publisher;
    case 'installDate':
      return p.installDate;
    case 'estimatedSize':
      return p.estimatedSize;
  }
}

/** Returns a new sorted array; the input is never mutated. */
export function sortPrograms(
  programs: readonly ProgramInfo[],
  field: SortField,
  order: SortOrder,
): ProgramInfo[] {
  return [...programs].sort((a, b) => {
    const primary = compareValues(valueFor(a, field), valueFor(b, field), order);
    // Stable tie-breakers so equal rows never shuffle between renders.
    if (primary !== 0) return primary;
    const byName = a.displayName.toLowerCase().localeCompare(b.displayName.toLowerCase());
    if (byName !== 0) return byName;
    return a.id.localeCompare(b.id);
  });
}

/** Cycles a column header: asc → desc → asc. */
export function nextSort(
  current: { field: SortField; order: SortOrder },
  field: SortField,
): { field: SortField; order: SortOrder } {
  if (current.field !== field) {
    // Text columns start ascending; size is most useful largest-first.
    return { field, order: field === 'estimatedSize' ? 'desc' : 'asc' };
  }
  return { field, order: current.order === 'asc' ? 'desc' : 'asc' };
}
