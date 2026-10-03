import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { ProgramInfo, SortField } from '@/types';
import { useProgramStore } from '@/stores/useProgramStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useT } from '@/i18n/useT';
import { formatDate, formatSize, orDash } from '@/lib/format';
import { filterPrograms, buildIndex } from '@/lib/search';
import { sortPrograms } from '@/lib/sort';
import { ProgramIcon } from './ProgramIcon';
import { ContextMenu, type ContextMenuItem } from './ContextMenu';
import { cn } from '@/lib/cn';

const COLUMNS: Array<{ field: SortField; labelKey: string; className: string }> = [
  { field: 'displayName', labelKey: 'table.name', className: 'flex-1 min-w-[240px]' },
  { field: 'displayVersion', labelKey: 'table.version', className: 'w-[120px]' },
  { field: 'publisher', labelKey: 'table.publisher', className: 'w-[200px]' },
  { field: 'installDate', labelKey: 'table.installDate', className: 'w-[120px]' },
  { field: 'estimatedSize', labelKey: 'table.size', className: 'w-[100px] text-right' },
];

interface ProgramTableProps {
  onUninstall: (program: ProgramInfo) => void;
  onCheckResidue: (program: ProgramInfo) => void;
  onOpenFolder: (program: ProgramInfo) => void;
  onCopyUninstall: (program: ProgramInfo) => void;
}

export function ProgramTable({
  onUninstall,
  onCheckResidue,
  onOpenFolder,
  onCopyUninstall,
}: ProgramTableProps) {
  const t = useT();
  const lang = useSettingsStore((s) => s.settings.language);
  const density = useSettingsStore((s) => s.settings.listDensity);

  const programs = useProgramStore((s) => s.programs);
  const loading = useProgramStore((s) => s.loading);
  const searchQuery = useProgramStore((s) => s.searchQuery);
  const sortField = useProgramStore((s) => s.sortField);
  const sortOrder = useProgramStore((s) => s.sortOrder);
  const selectedId = useProgramStore((s) => s.selectedId);
  const busyId = useProgramStore((s) => s.busyId);
  const setSort = useProgramStore((s) => s.setSort);
  const select = useProgramStore((s) => s.select);
  const setDetailOpen = useProgramStore((s) => s.setDetailOpen);
  const load = useProgramStore((s) => s.load);

  const parentRef = useRef<HTMLDivElement>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; program: ProgramInfo } | null>(null);

  // One pass builds the lower-cased search index for the whole scan.
  const index = useMemo(() => buildIndex(programs), [programs]);
  const visible = useMemo(
    () => sortPrograms(filterPrograms(programs, searchQuery, index), sortField, sortOrder),
    [programs, searchQuery, index, sortField, sortOrder],
  );

  const rowHeight = density === 'compact' ? 32 : 48;
  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    overscan: 12,
    getItemKey: (i) => visible[i]?.id ?? i,
  });

  // Keep the selection inside the filtered list.
  useEffect(() => {
    if (visible.length === 0) {
      if (selectedId !== null) select(null);
      return;
    }
    if (selectedId && !visible.some((p) => p.id === selectedId)) {
      select(visible[0].id);
    }
  }, [visible, selectedId, select]);

  const scrollToIndex = useCallback(
    (i: number) => {
      if (i < 0 || i >= visible.length) return;
      virtualizer.scrollToIndex(i, { align: 'auto' });
      select(visible[i].id);
    },
    [virtualizer, visible, select],
  );

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (visible.length === 0) return;
    const current = visible.findIndex((p) => p.id === selectedId);
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        scrollToIndex(current < 0 ? 0 : current + 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        scrollToIndex(current <= 0 ? 0 : current - 1);
        break;
      case 'Home':
        e.preventDefault();
        scrollToIndex(0);
        break;
      case 'End':
        e.preventDefault();
        scrollToIndex(visible.length - 1);
        break;
      case 'PageDown':
        e.preventDefault();
        scrollToIndex(current + Math.floor(400 / rowHeight));
        break;
      case 'PageUp':
        e.preventDefault();
        scrollToIndex(current - Math.floor(400 / rowHeight));
        break;
      case 'Enter': {
        const p = visible[current];
        if (p) {
          e.preventDefault();
          select(p.id);
          setDetailOpen(true);
        }
        break;
      }
      case 'Delete': {
        const p = visible[current];
        if (p?.canUninstall) {
          e.preventDefault();
          onUninstall(p);
        }
        break;
      }
      default:
        break;
    }
  };

  const openMenu = (e: React.MouseEvent, program: ProgramInfo) => {
    e.preventDefault();
    e.stopPropagation();
    select(program.id);
    setMenu({ x: e.clientX, y: e.clientY, program });
  };

  const menuItems: ContextMenuItem[] = useMemo(() => {
    if (!menu) return [];
    const p = menu.program;
    return [
      {
        id: 'uninstall',
        label: t('ctx.uninstall'),
        disabled: !p.canUninstall,
        danger: true,
        hint: 'Del',
        onSelect: () => onUninstall(p),
      },
      { id: 'sep1', label: '', separator: true },
      {
        id: 'details',
        label: t('ctx.details'),
        onSelect: () => {
          select(p.id);
          setDetailOpen(true);
        },
      },
      {
        id: 'copy',
        label: t('ctx.copyUninstall'),
        disabled: !p.uninstallString,
        onSelect: () => onCopyUninstall(p),
      },
      {
        id: 'folder',
        label: p.installLocation ? t('ctx.openFolder') : t('ctx.noFolder'),
        disabled: !p.installLocation,
        onSelect: () => onOpenFolder(p),
      },
      { id: 'sep2', label: '', separator: true },
      {
        id: 'residue',
        label: t('ctx.checkResidue'),
        onSelect: () => onCheckResidue(p),
      },
      {
        id: 'refresh',
        label: t('ctx.refresh'),
        onSelect: () => void load(),
      },
    ];
  }, [menu, t, onUninstall, onCopyUninstall, onOpenFolder, onCheckResidue, select, setDetailOpen, load]);

  const hasPrograms = programs.length > 0;
  const isFiltered = searchQuery.trim().length > 0;

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      {/* Sticky header */}
      <div
        role="row"
        className="flex shrink-0 items-center pr-3 text-[12px] font-medium"
        style={{
          height: 36,
          background: 'var(--win-surface)',
          borderBottom: '1px solid var(--win-border)',
          color: 'var(--win-text-secondary)',
        }}
      >
        {COLUMNS.map((col) => {
          const isActive = sortField === col.field;
          return (
            <button
              key={col.field}
              type="button"
              role="columnheader"
              aria-sort={
                isActive ? (sortOrder === 'asc' ? 'ascending' : 'descending') : 'none'
              }
              onClick={() => setSort(col.field)}
              className={cn(
                'flex h-full items-center gap-1 px-3 hover:text-[var(--win-text)]',
                col.className,
                col.field === 'estimatedSize' && 'justify-end',
              )}
              title={
                isActive
                  ? sortOrder === 'asc'
                    ? t('table.sortAsc')
                    : t('table.sortDesc')
                  : t('table.sortNone')
              }
            >
              <span className="truncate">{t(col.labelKey)}</span>
              {isActive && (
                <svg width="8" height="10" viewBox="0 0 8 10" aria-hidden="true">
                  <path
                    d={sortOrder === 'asc' ? 'M4 0l3 4H1z' : 'M4 10L1 6h6z'}
                    fill="currentColor"
                  />
                </svg>
              )}
            </button>
          );
        })}
      </div>

      {/* Virtualized body */}
      <div
        ref={parentRef}
        className="scroll-area min-h-0 flex-1 outline-none"
        role="grid"
        aria-label={t('app.title')}
        aria-rowcount={visible.length}
        tabIndex={0}
        onKeyDown={onKeyDown}
        onContextMenu={(e) => {
          if (e.target === e.currentTarget) {
            e.preventDefault();
            setMenu(null);
          }
        }}
      >
        {visible.length === 0 ? (
          <EmptyBody hasPrograms={hasPrograms} filtered={isFiltered} loading={loading} onLoad={load} />
        ) : (
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
            {virtualizer.getVirtualItems().map((item) => {
              const p = visible[item.index];
              if (!p) return null;
              const selected = p.id === selectedId;
              const busy = p.id === busyId;
              return (
                <div
                  key={item.key}
                  role="row"
                  aria-selected={selected}
                  className={cn(
                    'virt-row row absolute left-0 flex w-full cursor-default items-center pr-3',
                    busy && 'opacity-60',
                  )}
                  style={{
                    transform: `translateY(${item.start}px)`,
                    borderBottom: '1px solid var(--win-border)',
                  }}
                  onMouseDown={() => select(p.id)}
                  onDoubleClick={() => {
                    select(p.id);
                    setDetailOpen(true);
                  }}
                  onContextMenu={(e) => openMenu(e, p)}
                >
                  <div className="flex min-w-0 flex-1 items-center gap-2.5 px-3">
                    <ProgramIcon program={p} size={density === 'compact' ? 18 : 26} />
                    <span className="flex min-w-0 flex-1 items-center gap-1.5">
                      <span className="truncate" title={p.displayName}>
                        {p.displayName}
                      </span>
                      {p.isSystem && (
                        <Badge label={t('table.systemBadge')} tone="muted" />
                      )}
                      {p.parentKeyName && <Badge label={t('table.patchBadge')} tone="muted" />}
                    </span>
                  </div>
                  <div className="w-[120px] truncate px-3 text-[13px] tabular">
                    {orDash(p.displayVersion)}
                  </div>
                  <div
                    className="w-[200px] truncate px-3 text-[13px]"
                    style={{ color: 'var(--win-text-secondary)' }}
                    title={p.publisher}
                  >
                    {orDash(p.publisher)}
                  </div>
                  <div className="w-[120px] px-3 text-[13px] tabular">
                    {formatDate(p.installDate, lang)}
                  </div>
                  <div className="w-[100px] px-3 text-right text-[13px] tabular">
                    {formatSize(p.estimatedSize)}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {menu && (
        <ContextMenu
          items={menuItems}
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  );
}

function Badge({ label, tone }: { label: string; tone: 'muted' | 'accent' }) {
  return (
    <span
      className="shrink-0 rounded-[3px] px-1.5 py-px text-[10px] leading-4"
      style={{
        background:
          tone === 'accent'
            ? 'color-mix(in srgb, var(--win-accent) 16%, transparent)'
            : 'color-mix(in srgb, currentColor 10%, transparent)',
        color: 'var(--win-text-secondary)',
      }}
    >
      {label}
    </span>
  );
}

function EmptyBody({
  hasPrograms,
  filtered,
  loading,
  onLoad,
}: {
  hasPrograms: boolean;
  filtered: boolean;
  loading: boolean;
  onLoad: () => Promise<void>;
}) {
  const t = useT();
  if (loading) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3">
        <svg width="28" height="28" viewBox="0 0 24 24" className="spinner" aria-hidden="true">
          <circle cx="12" cy="12" r="9" fill="none" stroke="var(--win-border-strong)" strokeWidth="2" />
          <path d="M12 3a9 9 0 0 1 9 9" fill="none" stroke="var(--win-accent)" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <p className="text-[13px]" style={{ color: 'var(--win-text-secondary)' }}>
          {t('status.loading')}
        </p>
      </div>
    );
  }
  const showMatch = hasPrograms && filtered;
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-8 text-center">
      <p className="text-[14px] font-medium">
        {showMatch ? t('table.noMatchTitle') : t('table.emptyTitle')}
      </p>
      <p className="text-[13px]" style={{ color: 'var(--win-text-secondary)' }}>
        {showMatch ? t('table.noMatchBody') : t('table.emptyBody')}
      </p>
      {!showMatch && (
        <button
          type="button"
          onClick={() => void onLoad()}
          className="mt-2 rounded-[4px] px-3 py-1.5 text-[13px] text-white"
          style={{ background: 'var(--win-accent)' }}
        >
          {t('toolbar.refresh')}
        </button>
      )}
    </div>
  );
}
