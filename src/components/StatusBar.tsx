import { useProgramStore } from '@/stores/useProgramStore';
import { useT } from '@/i18n/useT';
import { formatDuration } from '@/lib/format';

export function StatusBar() {
  const t = useT();

  const programs = useProgramStore((s) => s.programs);
  const stats = useProgramStore((s) => s.stats);
  const elapsedMs = useProgramStore((s) => s.elapsedMs);
  const loading = useProgramStore((s) => s.loading);
  const searchQuery = useProgramStore((s) => s.searchQuery);
  const lastResult = useProgramStore((s) => s.lastResult);

  const filtered = searchQuery.trim().length > 0;
  const shown = filtered ? countMatches(programs, searchQuery) : programs.length;

  let text: string;
  if (loading) {
    text = t('status.loading');
  } else if (programs.length === 0) {
    text = t('status.idle');
  } else if (filtered) {
    text = t('status.filtered', { shown, total: programs.length });
  } else {
    text = t('status.loaded', {
      count: programs.length,
      time: formatDuration(elapsedMs),
    });
  }

  return (
    <footer
      className="flex shrink-0 items-center gap-3 px-3 text-[11px]"
      style={{
        height: 'var(--statusbar-h)',
        background: 'var(--win-surface)',
        borderTop: '1px solid var(--win-border)',
        color: 'var(--win-text-secondary)',
      }}
    >
      <span className="truncate">{text}</span>

      {stats && stats.deduplicated + stats.skippedNoName + stats.skippedInaccessible > 0 && (
        <span className="hidden truncate sm:inline" title={t('status.stats')}>
          {t('status.stats', {
            noName: stats.skippedNoName,
            noAccess: stats.skippedInaccessible,
            dedup: stats.deduplicated,
          })}
        </span>
      )}

      <div className="flex-1" />

      {lastResult && (
        <span
          className="truncate"
          style={{
            color:
              lastResult.status === 'failed'
                ? 'var(--win-danger)'
                : lastResult.status === 'completed'
                  ? 'var(--win-accent)'
                  : undefined,
          }}
        >
          {t(
            lastResult.status === 'completed'
              ? 'uninstall.result.completed'
              : lastResult.status === 'started'
                ? 'uninstall.result.started'
                : 'uninstall.result.failed',
          )}
          {lastResult.exitCode !== undefined && ` (${lastResult.exitCode})`}
        </span>
      )}
      <span className="tabular opacity-70">v{__APP_VERSION__}</span>
    </footer>
  );
}

function countMatches(
  programs: { id: string; displayName: string; publisher?: string; displayVersion?: string }[],
  query: string,
): number {
  const keys = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  return programs.filter((p) => {
    const hay = `${p.displayName} ${p.publisher ?? ''} ${p.displayVersion ?? ''}`.toLowerCase();
    return keys.every((k) => hay.includes(k));
  }).length;
}
