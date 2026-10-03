import { useEffect, useRef } from 'react';
import { useProgramStore } from '@/stores/useProgramStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useT } from '@/i18n/useT';
import { Button } from './Button';

interface ToolbarProps {
  onOpenSettings: () => void;
}

export function Toolbar({ onOpenSettings }: ToolbarProps) {
  const t = useT();
  const searchInput = useRef<HTMLInputElement>(null);

  const loading = useProgramStore((s) => s.loading);
  const load = useProgramStore((s) => s.load);
  const searchQuery = useProgramStore((s) => s.searchQuery);
  const setSearchQuery = useProgramStore((s) => s.setSearchQuery);

  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  // Toggling a scan filter must re-scan; the backend is what filters.
  const toggleSystemComponents = async (value: boolean) => {
    await update({ showSystemComponents: value });
    await load();
  };

  // Ctrl+F focuses the search box, Escape clears it.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        searchInput.current?.focus();
        searchInput.current?.select();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  return (
    <div
      className="flex shrink-0 items-center gap-2 px-3"
      style={{ height: 'var(--toolbar-h)', background: 'var(--win-surface)' }}
    >
      <div className="relative min-w-0 flex-1 max-w-[420px]">
        <svg
          className="pointer-events-none absolute top-1/2 -translate-y-1/2"
          style={{ left: 9, color: 'var(--win-text-secondary)' }}
          width="14"
          height="14"
          viewBox="0 0 16 16"
          aria-hidden="true"
        >
          <circle cx="7" cy="7" r="5" fill="none" stroke="currentColor" strokeWidth="1.4" />
          <path d="M11 11l4 4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
        </svg>
        <input
          ref={searchInput}
          type="search"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape' && searchQuery) {
              e.stopPropagation();
              setSearchQuery('');
            }
          }}
          placeholder={t('toolbar.searchPlaceholder')}
          aria-label={t('toolbar.search')}
          className="h-8 w-full rounded-[4px] pl-7 pr-2 text-[13px] outline-none"
          style={{
            background: 'var(--win-bg)',
            border: '1px solid var(--win-border-strong)',
            color: 'var(--win-text)',
          }}
        />
      </div>

      <label className="flex cursor-pointer select-none items-center gap-1.5 px-1 text-[13px]">
        <input
          type="checkbox"
          checked={settings.showSystemComponents}
          onChange={(e) => void toggleSystemComponents(e.target.checked)}
        />
        {t('toolbar.showSystemComponents')}
      </label>

      <div className="flex-1" />

      <Button onClick={() => void load()} disabled={loading} title={t('toolbar.refresh')}>
        <svg
          width="13"
          height="13"
          viewBox="0 0 16 16"
          className={loading ? 'spinner' : undefined}
          aria-hidden="true"
        >
          <path
            d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2v3.5H10"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.4"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        {loading ? t('toolbar.refreshing') : t('toolbar.refresh')}
      </Button>

      <Button onClick={onOpenSettings} title="Alt+S">
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="2.4" fill="none" stroke="currentColor" strokeWidth="1.3" />
          <path
            d="M8 1v2M8 13v2M1 8h2M13 8h2M3.2 3.2l1.4 1.4M11.4 11.4l1.4 1.4M12.8 3.2l-1.4 1.4M4.6 11.4l-1.4 1.4"
            stroke="currentColor"
            strokeWidth="1.3"
            strokeLinecap="round"
          />
        </svg>
        {t('toolbar.settings')}
      </Button>
    </div>
  );
}
