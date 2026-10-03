import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { call } from '@/lib/invoke';
import type { ProgramInfo, ResidueReport, UninstallResult } from '@/types';
import { useProgramStore } from '@/stores/useProgramStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useT } from '@/i18n/useT';
import { Toolbar } from '@/components/Toolbar';
import { ProgramTable } from '@/components/ProgramTable';
import { ProgramDetail } from '@/components/ProgramDetail';
import { StatusBar } from '@/components/StatusBar';
import { ConfirmUninstallDialog } from '@/components/ConfirmUninstallDialog';
import { ResidueDialog } from '@/components/ResidueDialog';
import { SettingsDialog } from '@/components/SettingsDialog';
import { ErrorState } from '@/components/ErrorState';

export default function App() {
  const t = useT();

  const loadPrograms = useProgramStore((s) => s.load);
  const programs = useProgramStore((s) => s.programs);
  const loadError = useProgramStore((s) => s.error);
  const settings = useSettingsStore((s) => s.settings);
  const settingsReady = useSettingsStore((s) => s.ready);
  const loadSettings = useSettingsStore((s) => s.load);

  const [uninstallTarget, setUninstallTarget] = useState<ProgramInfo | null>(null);
  const [residue, setResidue] = useState<{ program: ProgramInfo; report: ResidueReport } | null>(
    null,
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [toast, setToast] = useState<{ id: number; text: string; tone: 'info' | 'error' } | null>(
    null,
  );

  // ---- startup: settings first, because they drive the scan filters --------
  useEffect(() => {
    void (async () => {
      await loadSettings();
      await loadPrograms();
    })();
  }, [loadSettings, loadPrograms]);

  // ---- theme, density, tray rescan ---------------------------------------
  useEffect(() => {
    const apply = () => {
      const dark =
        settings.theme === 'dark' ||
        (settings.theme === 'system' &&
          window.matchMedia('(prefers-color-scheme: dark)').matches);
      document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    };
    apply();
    if (settings.theme !== 'system') return;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  }, [settings.theme]);

  useEffect(() => {
    document.documentElement.dataset.density = settings.listDensity;
  }, [settings.listDensity]);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let cancelled = false;
    void listen('app://rescan', () => void loadPrograms()).then((unlisten) => {
      if (cancelled) unlisten();
      else dispose = unlisten;
    });
    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [loadPrograms]);

  // ---- global shortcuts --------------------------------------------------
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.altKey && e.key.toLowerCase() === 's') {
        e.preventDefault();
        setSettingsOpen(true);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  const notify = useCallback((text: string, tone: 'info' | 'error' = 'info') => {
    setToast({ id: Date.now(), text, tone });
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 4000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  // ---- actions ------------------------------------------------------------
  const openFolder = useCallback(
    async (program: ProgramInfo) => {
      try {
        await call('open_install_location', { id: program.id });
      } catch (e) {
        const err = e as { message?: string };
        notify(err.message ?? t('detail.noInstallLocation'), 'error');
      }
    },
    [notify, t],
  );

  const copyUninstall = useCallback(
    async (program: ProgramInfo) => {
      try {
        await writeText(program.uninstallString ?? '');
        notify(t('detail.copied'));
      } catch {
        notify(t('detail.copyFailed', { error: 'clipboard' }), 'error');
      }
    },
    [notify, t],
  );

  const checkResidue = useCallback(
    async (program: ProgramInfo) => {
      try {
        const report = await call<ResidueReport>('check_residue', { id: program.id });
        setResidue({ program, report });
      } catch (e) {
        const err = e as { message?: string };
        notify(err.message ?? t('error.loadFailed'), 'error');
      }
    },
    [notify, t],
  );

  // With confirmation disabled, the dialog still opens so the command stays
  // visible (F-202) but starts the uninstall itself (§10.4).
  const handleUninstallRequest = useCallback(
    (program: ProgramInfo) => {
      if (!program.canUninstall) {
        notify(t('uninstall.noCommand'), 'error');
        return;
      }
      setUninstallTarget(program);
    },
    [notify, t],
  );

  const onUninstallDone = useCallback(
    (result: UninstallResult) => {
      const id = uninstallTarget?.id;
      setUninstallTarget(null);
      if (id) useProgramStore.getState().removeProgram(id);
      notify(result.message, result.status === 'failed' ? 'error' : 'info');
      // A residue report is only trustworthy after a fresh scan.
      if (result.residue) void loadPrograms();
    },
    [uninstallTarget, notify, loadPrograms],
  );

  // ---- render -------------------------------------------------------------
  const showBlockingError = settingsReady && loadError !== null && programs.length === 0;

  return (
    <div
      className="flex h-full flex-col"
      style={{ background: 'var(--win-bg)' }}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => e.preventDefault()}
    >
      <header
        className="flex shrink-0 items-center gap-2 px-3"
        style={{ height: 40, background: 'var(--win-surface)', borderBottom: '1px solid var(--win-border)' }}
      >
        <svg width="17" height="17" viewBox="0 0 24 24" aria-hidden="true">
          <rect width="24" height="24" rx="5" fill="var(--win-accent)" />
          <path d="M12 5v7M8.5 9.5L12 13l3.5-3.5M7 16h10" stroke="#fff" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" fill="none" />
        </svg>
        <div className="leading-tight">
          <h1 className="text-[13px] font-semibold">{t('app.title')}</h1>
          <p className="text-[10px]" style={{ color: 'var(--win-text-secondary)' }}>
            {t('app.subtitle')}
          </p>
        </div>
      </header>

      <Toolbar onOpenSettings={() => setSettingsOpen(true)} />

      <div className="flex min-h-0 flex-1">
        {showBlockingError ? (
          <ErrorState error={loadError} onRetry={() => void loadPrograms()} />
        ) : (
          <>
            <ProgramTable
              onUninstall={handleUninstallRequest}
              onCheckResidue={(p) => void checkResidue(p)}
              onOpenFolder={(p) => void openFolder(p)}
              onCopyUninstall={(p) => void copyUninstall(p)}
            />
            <div className="flex" style={{ borderLeft: '1px solid var(--win-border)' }}>
              <ProgramDetail
                onUninstall={handleUninstallRequest}
                onCheckResidue={(p) => void checkResidue(p)}
                onOpenFolder={(p) => void openFolder(p)}
                onCopyUninstall={(p) => void copyUninstall(p)}
              />
            </div>
          </>
        )}
      </div>

      <StatusBar />

      {/* `key` per program so the dialog's state resets naturally between
          targets instead of needing a reset effect. The prefix keeps the two
          dialogs' keys distinct while both are closed — a shared bare
          `'none'` would be a duplicate key among siblings. */}
      <ConfirmUninstallDialog
        key={uninstallTarget ? `uninstall:${uninstallTarget.id}` : 'uninstall:none'}
        program={uninstallTarget}
        autoStart={!settings.confirmBeforeUninstall}
        onClose={() => setUninstallTarget(null)}
        onDone={onUninstallDone}
      />

      <ResidueDialog
        key={residue ? `residue:${residue.program.id}` : 'residue:none'}
        program={residue?.program ?? null}
        report={residue?.report ?? null}
        onClose={() => {
          setResidue(null);
          void loadPrograms();
        }}
        onCleaned={() => void loadPrograms()}
      />

      <SettingsDialog key={settingsOpen ? 'open' : 'closed'} open={settingsOpen} onClose={() => setSettingsOpen(false)} />

      {toast && (
        <div
          key={toast.id}
          role="status"
          className="flyout fixed bottom-10 left-1/2 z-[70] max-w-[70vw] -translate-x-1/2 rounded-[6px] px-4 py-2 text-[12px] shadow-[var(--win-shadow-flyout)]"
          style={{
            background: 'var(--win-surface)',
            border: '1px solid var(--win-border-strong)',
            color: toast.tone === 'error' ? 'var(--win-danger)' : 'var(--win-text)',
          }}
        >
          {toast.text}
        </div>
      )}
    </div>
  );
}
