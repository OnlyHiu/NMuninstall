import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { ProgramInfo, UninstallResult } from '@/types';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useT } from '@/i18n/useT';
import { Dialog } from './Dialog';
import { Button } from './Button';
import { ProgramIcon } from './ProgramIcon';
import { call } from '@/lib/invoke';
import type { AppErrorPayload } from '@/types';

/** Phases the backend reports over `uninstall://progress` (技术文档 §7.8). */
type UninstallPhase = 'starting' | 'detecting_residue' | 'done';

const PHASE_LABEL: Record<UninstallPhase, string> = {
  starting: 'uninstall.progress.starting',
  detecting_residue: 'uninstall.progress.detecting',
  done: 'uninstall.progress.done',
};

interface ConfirmUninstallDialogProps {
  program: ProgramInfo | null;
  /**
   * When the user turned off "confirm before uninstalling", the dialog still
   * appears — so the command is visible (F-202) — but it starts immediately
   * and the cancel button becomes a "hide" action (§10.4).
   */
  autoStart?: boolean;
  onClose: () => void;
  onDone: (result: UninstallResult) => void;
}

/** Previews the command the backend will actually run (F-202). */
interface Preview {
  executable: string;
  args: string[];
  kind: 'msi' | 'other';
  canWait: boolean;
}

/**
 * Client-side mirror of `src-tauri/src/uninstaller/parser.rs` for preview only.
 * The backend re-parses and is authoritative; this exists so the user sees the
 * resolved command *before* confirming.
 */
function previewCommand(
  uninstall: string | undefined,
  quiet: string | undefined,
  wantQuiet: boolean,
): Preview | null {
  const source = (wantQuiet && quiet?.trim()) || uninstall?.trim();
  if (!source) return null;
  const isMsi = /msiexec/i.test(source);
  if (isMsi) {
    const guid = source.match(/\{[^}]+\}/)?.[0];
    if (!guid) return null;
    const args = ['/X', guid];
    if (/\/qn|quiet|passive/i.test(source)) args.push('/qn');
    args.push('/norestart');
    return { executable: 'msiexec.exe', args, kind: 'msi', canWait: true };
  }
  const quoted = source.startsWith('"');
  let executable: string;
  let rest: string;
  if (quoted) {
    const end = source.indexOf('"', 1);
    if (end < 0) return null;
    executable = source.slice(1, end);
    rest = source.slice(end + 1).trim();
  } else {
    // Prefer a split that lands on an executable suffix.
    const parts = source.split(/\s+/);
    let cut = 0;
    for (let i = 0; i < parts.length; i += 1) {
      if (/\.(exe|com|bat|cmd)$/i.test(parts[i])) {
        cut = i;
        break;
      }
      cut = i;
    }
    executable = parts.slice(0, cut + 1).join(' ');
    rest = parts.slice(cut + 1).join(' ');
  }
  const args = rest ? rest.match(/"[^"]*"|\S+/g)?.map((a) => a.replace(/^"|"$/g, '')) ?? [] : [];
  const base = executable.split(/[\\/]/).pop()?.toLowerCase() ?? '';
  return {
    executable,
    args,
    kind: 'other',
    canWait: base === 'msiexec.exe' || base === 'rundll32.exe',
  };
}

export function ConfirmUninstallDialog({
  program,
  autoStart = false,
  onClose,
  onDone,
}: ConfirmUninstallDialogProps) {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const [quiet, setQuiet] = useState(settings.defaultQuietUninstall);
  const [busy, setBusy] = useState(false);
  const [phase, setPhase] = useState<UninstallPhase>('starting');
  const [error, setError] = useState<AppErrorPayload | null>(null);

  // All derived values are computed before the early return so hook order
  // stays stable across renders.
  const hasQuiet = Boolean(program?.quietUninstallString?.trim());
  // `/qn` is only injected for MSI, so the checkbox is meaningless otherwise.
  const quietUsable = hasQuiet || Boolean(program?.windowsInstaller);
  const preview = program
    ? previewCommand(program.uninstallString, program.quietUninstallString, quiet)
    : null;
  const quietFallsBack = quiet && !hasQuiet && !program?.windowsInstaller;

  const start = async () => {
    if (!program) return;
    setBusy(true);
    setPhase('starting');
    setError(null);
    try {
      const result = await call<UninstallResult>('uninstall_program', {
        id: program.id,
        quiet,
        checkResidue: settings.checkResidueAfterUninstall,
      });
      onDone(result);
    } catch (e) {
      setError(e as AppErrorPayload);
      setBusy(false);
    }
  };

  // The backend reports each stage over `uninstall://progress`; the dialog
  // shows it so a long-running uninstaller does not look frozen. Events for
  // other programs are ignored — one dialog handles one program at a time.
  useEffect(() => {
    const id = program?.id;
    if (!id) return;
    let dispose: (() => void) | undefined;
    let cancelled = false;
    void listen<{ id: string; phase: string }>('uninstall://progress', (e) => {
      if (e.payload.id !== id) return;
      if (e.payload.phase in PHASE_LABEL) {
        setPhase(e.payload.phase as UninstallPhase);
      }
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else dispose = unlisten;
    });
    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [program?.id]);

  // Firing the uninstaller once per program is exactly the "synchronise with
  // an external system" case effects exist for; the parent remounts this
  // component with `key={program.id}`, so this runs a single time.
  useEffect(() => {
    if (autoStart && program && preview) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- see above
      void start();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- mount-once by design
  }, []);

  if (!program) return null;

  return (
    <Dialog
      open
      title={t('uninstall.title')}
      onClose={busy ? () => {} : onClose}
      dismissOnBackdrop={!busy}
      width={500}
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>
            {busy ? t('uninstall.progress.starting') : t('uninstall.cancel')}
          </Button>
          <Button
            variant="danger"
            onClick={() => void start()}
            disabled={busy || !preview}
            hidden={autoStart}
          >
            {t('uninstall.start')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex items-start gap-3">
          <ProgramIcon program={program} size={32} />
          <div className="min-w-0">
            <p className="text-[15px] font-semibold break-words">
              {t('uninstall.confirm', { name: program.displayName })}
            </p>
            <p className="mt-1 text-[12px]" style={{ color: 'var(--win-text-secondary)' }}>
              {[program.displayVersion, program.publisher].filter(Boolean).join(' · ') || '—'}
            </p>
          </div>
        </div>

        {program.isSystem && (
          <div
            className="rounded-[4px] px-3 py-2 text-[12px]"
            style={{
              background: 'var(--win-warning-bg)',
              border: '1px solid var(--win-warning-border)',
              color: 'var(--win-warning-text)',
            }}
            role="alert"
          >
            {t('uninstall.systemWarning')}
          </div>
        )}

        {preview ? (
          <div>
            <div className="mb-1.5 text-[12px]" style={{ color: 'var(--win-text-secondary)' }}>
              {t('uninstall.command')}
            </div>
            <div
              className="mono select-all break-all rounded-[4px] px-2.5 py-2"
              style={{ background: 'var(--win-bg)', border: '1px solid var(--win-border)' }}
            >
              <span className="select-none" style={{ color: 'var(--win-text-secondary)' }}>
                &gt;&nbsp;
              </span>
              {preview.executable}
              {preview.args.length > 0 && ` ${preview.args.join(' ')}`}
            </div>
            {!preview.canWait && (
              <p className="mt-1.5 text-[11px]" style={{ color: 'var(--win-text-secondary)' }}>
                {t('uninstall.progress.starting')}
              </p>
            )}
          </div>
        ) : (
          <div
            className="rounded-[4px] px-3 py-2 text-[12px]"
            style={{
              background: 'var(--win-warning-bg)',
              border: '1px solid var(--win-warning-border)',
              color: 'var(--win-warning-text)',
            }}
          >
            {program.uninstallString
              ? t('error.detail')
              : t('uninstall.noCommand')}
          </div>
        )}

        <label
          className="flex items-start gap-2 text-[12px]"
          style={{ opacity: quietUsable ? 1 : 0.55 }}
        >
          <input
            type="checkbox"
            checked={quiet && quietUsable}
            disabled={!quietUsable || busy}
            onChange={(e) => setQuiet(e.target.checked)}
            className="mt-0.5"
          />
          <span>
            {t('uninstall.quiet')}
            {!quietUsable && (
              <span className="block" style={{ color: 'var(--win-text-secondary)' }}>
                {t('uninstall.quietUnavailable')}
              </span>
            )}
            {quietFallsBack && (
              <span className="block" style={{ color: 'var(--win-text-secondary)' }}>
                {t('uninstall.quietUnavailable')}
              </span>
            )}
          </span>
        </label>

        {busy && (
          <div className="flex items-center gap-2 text-[12px]" style={{ color: 'var(--win-text-secondary)' }}>
            <svg width="14" height="14" viewBox="0 0 24 24" className="spinner" aria-hidden="true">
              <circle cx="12" cy="12" r="9" fill="none" stroke="var(--win-border-strong)" strokeWidth="3" />
              <path d="M12 3a9 9 0 0 1 9 9" fill="none" stroke="var(--win-accent)" strokeWidth="3" strokeLinecap="round" />
            </svg>
            {t(PHASE_LABEL[phase])}
          </div>
        )}

        {error && (
          <div
            className="rounded-[4px] px-3 py-2 text-[12px]"
            style={{ background: 'var(--win-warning-bg)', border: '1px solid var(--win-warning-border)', color: 'var(--win-danger)' }}
            role="alert"
          >
            {error.message}
            {error.detail && (
              <div className="mono mt-1 opacity-80">{error.detail}</div>
            )}
          </div>
        )}
      </div>
    </Dialog>
  );
}
