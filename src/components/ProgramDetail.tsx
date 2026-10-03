import { useProgramStore } from '@/stores/useProgramStore';
import { useT, useLang } from '@/i18n/useT';
import { formatDate, formatSize, orDash } from '@/lib/format';
import { ProgramIcon } from './ProgramIcon';
import { Button } from './Button';
import type { ProgramInfo } from '@/types';

interface ProgramDetailProps {
  onUninstall: (program: ProgramInfo) => void;
  onCheckResidue: (program: ProgramInfo) => void;
  onOpenFolder: (program: ProgramInfo) => void;
  onCopyUninstall: (program: ProgramInfo) => void;
}

export function ProgramDetail({
  onUninstall,
  onCheckResidue,
  onOpenFolder,
  onCopyUninstall,
}: ProgramDetailProps) {
  const t = useT();
  const lang = useLang();
  const programs = useProgramStore((s) => s.programs);
  const selectedId = useProgramStore((s) => s.selectedId);
  const open = useProgramStore((s) => s.detailOpen);
  const setOpen = useProgramStore((s) => s.setDetailOpen);

  const program = programs.find((p) => p.id === selectedId) ?? null;

  if (!open) {
    return (
      <aside
        className="flex shrink-0 flex-col items-center justify-center gap-3 border-l p-6 text-center"
        style={{ width: 'var(--detail-w)', borderColor: 'var(--win-border)' }}
      >
        <svg width="34" height="34" viewBox="0 0 24 24" aria-hidden="true" opacity="0.35">
          <rect x="3" y="4" width="18" height="5" rx="1.2" fill="none" stroke="currentColor" strokeWidth="1.3" />
          <rect x="3" y="12" width="18" height="8" rx="1.2" fill="none" stroke="currentColor" strokeWidth="1.3" />
          <path d="M6 6.5h.01M6 16h.01" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <p className="text-[12px]" style={{ color: 'var(--win-text-secondary)' }}>
          {t('detail.empty')}
        </p>
      </aside>
    );
  }

  return (
    <aside
      className="flex min-h-0 shrink-0 flex-col overflow-y-auto"
      style={{ width: 'var(--detail-w)', borderColor: 'var(--win-border)' }}
    >
      <header
        className="flex items-center justify-between px-4 py-3"
        style={{ borderBottom: '1px solid var(--win-border)' }}
      >
        <h2 className="text-[13px] font-semibold">{t('detail.title')}</h2>
        <button
          type="button"
          onClick={() => setOpen(false)}
          aria-label={t('detail.close')}
          className="flex size-7 items-center justify-center rounded hover:bg-black/5 dark:hover:bg-white/10"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
            <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
          </svg>
        </button>
      </header>

      {!program ? (
        <p
          className="px-4 py-6 text-[12px]"
          style={{ color: 'var(--win-text-secondary)' }}
        >
          {t('detail.empty')}
        </p>
      ) : (
        <div className="flex flex-col gap-4 p-4">
          <div className="flex items-start gap-3">
            <ProgramIcon program={program} size={36} />
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-1.5">
                <h3 className="text-[15px] font-semibold break-words">{program.displayName}</h3>
                {program.isSystem && (
                  <span
                    className="rounded-[3px] px-1.5 py-px text-[10px]"
                    style={{
                      background: 'color-mix(in srgb, var(--win-text) 10%, transparent)',
                      color: 'var(--win-text-secondary)',
                    }}
                  >
                    {t('table.systemBadge')}
                  </span>
                )}
              </div>
              {program.publisher && (
                <p
                  className="mt-0.5 break-words text-[12px]"
                  style={{ color: 'var(--win-text-secondary)' }}
                >
                  {program.publisher}
                </p>
              )}
            </div>
          </div>

          <div
            className="flex flex-wrap gap-1.5"
            style={{ borderTop: '1px solid var(--win-border)', paddingTop: 12 }}
          >
            <Button variant="accent" disabled={!program.canUninstall} onClick={() => onUninstall(program)}>
              {t('ctx.uninstall')}
            </Button>
            <Button
              disabled={!program.installLocation}
              onClick={() => onOpenFolder(program)}
              title={program.installLocation ? program.installLocation : t('detail.noInstallLocation')}
            >
              {t('detail.openFolder')}
            </Button>
          </div>
          <div className="flex flex-wrap gap-1.5">
            <Button
              variant="subtle"
              disabled={!program.uninstallString}
              onClick={() => onCopyUninstall(program)}
            >
              {t('detail.copyUninstall')}
            </Button>
            <Button variant="subtle" onClick={() => onCheckResidue(program)}>
              {t('detail.checkResidue')}
            </Button>
          </div>

          <dl className="flex flex-col gap-2.5 text-[12px]">
            <Row label={t('detail.version')} value={orDash(program.displayVersion)} />
            <Row label={t('detail.publisher')} value={orDash(program.publisher)} />
            <Row label={t('detail.installDate')} value={formatDate(program.installDate, lang)} />
            <Row label={t('detail.size')} value={formatSize(program.estimatedSize)} />
            <Row
              label={t('detail.bitness')}
              value={`${program.is64Bit ? t('table.bitness64') : t('table.bitness32')} · ${program.hive}`}
            />
            <Row label={t('detail.installLocation')} value={orDash(program.installLocation)} mono />
          </dl>

          <div className="flex flex-col gap-2.5 text-[12px]">
            <Field label={t('detail.uninstallString')} value={program.uninstallString} />
            <Field label={t('detail.quietString')} value={program.quietUninstallString} />
            <Field label={t('detail.source')} value={program.regPath} />
          </div>
        </div>
      )}
    </aside>
  );
}

function Row({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid grid-cols-[92px_1fr] gap-2">
      <dt style={{ color: 'var(--win-text-secondary)' }}>{label}</dt>
      <dd className={mono ? 'mono break-all' : 'break-words'}>{value}</dd>
    </div>
  );
}

function Field({ label, value }: { label: string; value?: string }) {
  if (!value) return null;
  return (
    <div>
      <div className="mb-1" style={{ color: 'var(--win-text-secondary)' }}>
        {label}
      </div>
      <div
        className="mono select-all break-all rounded-[4px] px-2 py-1.5"
        style={{ background: 'var(--win-bg)', border: '1px solid var(--win-border)' }}
      >
        {value}
      </div>
    </div>
  );
}
