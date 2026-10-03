import { useEffect, useState } from 'react';
import type { AppInfo, AppSettings as AppSettingsType, Language, ListDensity, Theme } from '@/types';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useT } from '@/i18n/useT';
import { Dialog } from './Dialog';
import { Button } from './Button';
import { call } from '@/lib/invoke';
import { cn } from '@/lib/cn';

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

/**
 * `App` mounts this with `key={open ? 'open' : 'closed'}`, so the draft state
 * is initialised from the current settings exactly once per open — no reset
 * effect needed.
 */
export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [draft, setDraft] = useState<AppSettingsType>(settings);
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void call<AppInfo>('get_app_info')
      .then((value) => {
        if (!cancelled) setInfo(value);
      })
      .catch(() => {
        if (!cancelled) setInfo(null);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  const set = <K extends keyof AppSettingsType>(key: K, value: AppSettingsType[K]) =>
    setDraft((d) => ({ ...d, [key]: value }));

  const apply = async () => {
    await update(draft);
    onClose();
  };

  return (
    <Dialog
      open={open}
      title={t('settings.title')}
      onClose={onClose}
      width={520}
      footer={
        <>
          <Button onClick={onClose}>{t('settings.cancel')}</Button>
          <Button variant="accent" onClick={() => void apply()}>
            {t('settings.save')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-5 text-[13px]">
        <Section title={t('settings.appearance')}>
          <Choice
            label={t('settings.theme')}
            value={draft.theme}
            options={[
              { value: 'system', label: t('settings.themeSystem') },
              { value: 'light', label: t('settings.themeLight') },
              { value: 'dark', label: t('settings.themeDark') },
            ]}
            onChange={(v) => set('theme', v as Theme)}
          />
          <Choice
            label={t('settings.language')}
            value={draft.language}
            options={[
              { value: 'zh-CN', label: '简体中文' },
              { value: 'en-US', label: 'English (US)' },
            ]}
            onChange={(v) => set('language', v as Language)}
          />
          <Choice
            label={t('settings.density')}
            value={draft.listDensity}
            options={[
              { value: 'comfortable', label: t('settings.densityComfortable') },
              { value: 'compact', label: t('settings.densityCompact') },
            ]}
            onChange={(v) => set('listDensity', v as ListDensity)}
          />
        </Section>

        <Section title={t('settings.behaviour')}>
          <Toggle
            label={t('settings.showSystemComponents')}
            hint={t('settings.showSystemComponentsHint')}
            checked={draft.showSystemComponents}
            onChange={(v) => set('showSystemComponents', v)}
          />
          <Toggle
            label={t('settings.show32Bit')}
            checked={draft.show32BitPrograms}
            onChange={(v) => set('show32BitPrograms', v)}
          />
          <Toggle
            label={t('settings.confirmUninstall')}
            hint={t('settings.confirmUninstallHint')}
            checked={draft.confirmBeforeUninstall}
            onChange={(v) => set('confirmBeforeUninstall', v)}
          />
          <Toggle
            label={t('settings.checkResidue')}
            checked={draft.checkResidueAfterUninstall}
            onChange={(v) => set('checkResidueAfterUninstall', v)}
          />
          <Toggle
            label={t('settings.defaultQuiet')}
            checked={draft.defaultQuietUninstall}
            onChange={(v) => set('defaultQuietUninstall', v)}
          />
          <label className="flex flex-col gap-1">
            <span>{t('settings.timeout')}</span>
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={30}
                max={3600}
                step={30}
                value={draft.uninstallTimeoutSecs}
                onChange={(e) => set('uninstallTimeoutSecs', Number(e.target.value))}
                className="flex-1"
              />
              <input
                type="number"
                min={30}
                max={3600}
                value={draft.uninstallTimeoutSecs}
                onChange={(e) => set('uninstallTimeoutSecs', Number(e.target.value) || 300)}
                className="tabular w-20 rounded-[4px] px-2 py-1 text-right"
                style={{ background: 'var(--win-bg)', border: '1px solid var(--win-border-strong)' }}
              />
            </div>
            <span className="text-[11px]" style={{ color: 'var(--win-text-secondary)' }}>
              {t('settings.timeoutHint')}
            </span>
          </label>
        </Section>

        <Section title={t('settings.about')}>
          <div className="flex flex-col gap-1" style={{ color: 'var(--win-text-secondary)' }}>
            <span className="tabular">
              {t('settings.version')}: {info?.version ?? '—'} · Tauri {info?.tauriVersion ?? '—'} ·{' '}
              {info?.os ?? '—'}/{info?.arch ?? '—'}
            </span>
            <span className="mono break-all">{info?.configPath ?? ''}</span>
            <Button
              variant="subtle"
              className="self-start"
              onClick={() => void call('open_log_dir').catch(() => undefined)}
            >
              {t('settings.openLogs')}
            </Button>
          </div>
        </Section>
      </div>
    </Dialog>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h3
        className="text-[12px] font-semibold"
        style={{ color: 'var(--win-text-secondary)' }}
      >
        {title}
      </h3>
      {children}
    </section>
  );
}

function Toggle({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-start gap-2.5">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-1"
      />
      <span>
        {label}
        {hint && (
          <span className="block text-[11px]" style={{ color: 'var(--win-text-secondary)' }}>
            {hint}
          </span>
        )}
      </span>
    </label>
  );
}

function Choice({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <span>{label}</span>
      <div className="flex gap-1 rounded-[4px] p-0.5" style={{ background: 'var(--win-bg)' }}>
        {options.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => onChange(opt.value)}
            className={cn(
              'flex-1 rounded-[3px] px-2 py-1 text-[12px]',
              value === opt.value ? 'font-medium shadow-sm' : '',
            )}
            style={
              value === opt.value
                ? { background: 'var(--win-surface)' }
                : { color: 'var(--win-text-secondary)' }
            }
          >
            {opt.label}
          </button>
        ))}
      </div>
    </div>
  );
}
