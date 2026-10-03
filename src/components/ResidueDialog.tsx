import { useState } from 'react';
import type { ProgramInfo, ResidueItem, ResidueReport, CleanResult } from '@/types';
import { useT } from '@/i18n/useT';
import { Dialog } from './Dialog';
import { Button } from './Button';
import { formatSize, orDash } from '@/lib/format';
import { call } from '@/lib/invoke';
import { cn } from '@/lib/cn';

interface ResidueDialogProps {
  program: ProgramInfo | null;
  report: ResidueReport | null;
  onClose: () => void;
  onCleaned: (result: CleanResult) => void;
}

export function ResidueDialog({ program, report, onClose, onCleaned }: ResidueDialogProps) {
  const t = useT();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<CleanResult | null>(null);

  if (!program) return null;

  const all: ResidueItem[] = [...(report?.registryKeys ?? []), ...(report?.installDirs ?? [])];
  const removable = all.filter((i) => i.removable);
  const totalBytes = removable.reduce((sum, i) => sum + (i.sizeBytes ?? 0), 0);
  const totalFiles = removable.reduce((sum, i) => sum + (i.fileCount ?? 0), 0);

  const toggle = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const toggleAll = () => {
    setSelected((prev) =>
      prev.size === removable.length
        ? new Set()
        : new Set(removable.map((i) => `${i.kind}:${i.id}`)),
    );
  };

  const clean = async () => {
    if (!report) return;
    setBusy(true);
    try {
      const registryKeys = removable
        .filter((i) => i.kind === 'registry' && selected.has(`${i.kind}:${i.id}`))
        .map((i) => i.id);
      const directories = removable
        .filter((i) => i.kind === 'directory' && selected.has(`${i.kind}:${i.id}`))
        .map((i) => i.id);
      const res = await call<CleanResult>('clean_residue', {
        id: program.id,
        registryKeys,
        directories,
        confirmed: true,
      });
      setResult(res);
      setConfirming(false);
      onCleaned(res);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open
      title={t('residue.title')}
      onClose={onClose}
      width={560}
      footer={
        result ? (
          <Button variant="accent" onClick={onClose}>
            {t('common.close')}
          </Button>
        ) : (
          <>
            <Button onClick={onClose}>{t('common.close')}</Button>
            <Button
              variant="danger"
              disabled={selected.size === 0 || busy}
              onClick={() => setConfirming(true)}
            >
              {t('residue.deleteSelected', { count: selected.size })}
            </Button>
          </>
        )
      }
    >
      {result ? (
        <div className="flex flex-col gap-3 text-[13px]">
          <p>
            {result.failed.length === 0
              ? t('residue.deleted', { count: result.removed.length })
              : t('residue.deletePartial', {
                  ok: result.removed.length,
                  failed: result.failed.length,
                })}
          </p>
          {result.failed.length > 0 && (
            <ul className="mono flex flex-col gap-1 text-[11px]" style={{ color: 'var(--win-danger)' }}>
              {result.failed.map((f) => (
                <li key={f.path} className="break-all">
                  {f.path} — {f.error}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : !report || all.length === 0 ? (
        <p className="text-[13px]">{t('residue.none')}</p>
      ) : (
        <div className="flex flex-col gap-4">
          <p className="text-[13px]">{t('residue.subtitle', { name: program.displayName })}</p>

          {removable.length > 0 && (
            <div className="flex items-center justify-between text-[12px]">
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={selected.size === removable.length && removable.length > 0}
                  onChange={toggleAll}
                />
                {t('residue.selectAll')}
              </label>
              <span style={{ color: 'var(--win-text-secondary)' }}>
                {t('residue.totalSize', { size: formatSize(totalBytes), files: totalFiles })}
              </span>
            </div>
          )}

          {report.registryKeys.length > 0 && (
            <Group title={t('residue.registry')}>
              {report.registryKeys.map((item) => (
                <ResidueRow
                  key={`${item.kind}:${item.id}`}
                  item={item}
                  checked={selected.has(`${item.kind}:${item.id}`)}
                  onToggle={() => toggle(`${item.kind}:${item.id}`)}
                />
              ))}
            </Group>
          )}

          {report.installDirs.length > 0 && (
            <Group title={t('residue.directories')}>
              {report.installDirs.map((item) => (
                <ResidueRow
                  key={`${item.kind}:${item.id}`}
                  item={item}
                  checked={selected.has(`${item.kind}:${item.id}`)}
                  onToggle={() => toggle(`${item.kind}:${item.id}`)}
                />
              ))}
            </Group>
          )}

          {confirming && (
            <div
              className="rounded-[4px] px-3 py-2.5 text-[12px]"
              style={{
                background: 'var(--win-warning-bg)',
                border: '1px solid var(--win-warning-border)',
                color: 'var(--win-warning-text)',
              }}
              role="alert"
            >
              <p className="font-medium">{t('residue.deleteConfirmTitle')}</p>
              <p className="mt-1">{t('residue.deleteConfirmBody')}</p>
              <ul className="mono mt-1.5 flex flex-col gap-0.5 text-[11px]">
                {removable
                  .filter((i) => selected.has(`${i.kind}:${i.id}`))
                  .map((i) => (
                    <li key={`${i.kind}:${i.id}`} className="break-all">
                      {i.path}
                    </li>
                  ))}
              </ul>
              <div className="mt-2.5 flex justify-end gap-2">
                <Button onClick={() => setConfirming(false)} disabled={busy}>
                  {t('uninstall.cancel')}
                </Button>
                <Button variant="danger" onClick={() => void clean()} disabled={busy}>
                  {t('common.ok')}
                </Button>
              </div>
            </div>
          )}
        </div>
      )}
    </Dialog>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section>
      <h3 className="mb-1.5 text-[12px] font-medium" style={{ color: 'var(--win-text-secondary)' }}>
        {title}
      </h3>
      <ul className="flex flex-col gap-1">{children}</ul>
    </section>
  );
}

function ResidueRow({
  item,
  checked,
  onToggle,
}: {
  item: ResidueItem;
  checked: boolean;
  onToggle: () => void;
}) {
  const t = useT();
  const meta = [
    item.sizeBytes !== undefined ? formatSize(item.sizeBytes) : undefined,
    item.fileCount !== undefined ? `${item.fileCount} ${t('residue.directories')}` : undefined,
  ]
    .filter(Boolean)
    .join(' · ');

  return (
    <li
      className={cn(
        'flex items-start gap-2 rounded-[4px] px-2 py-1.5 text-[12px]',
        item.removable ? 'cursor-pointer hover:bg-black/5 dark:hover:bg-white/10' : 'opacity-70',
      )}
      onClick={item.removable ? onToggle : undefined}
    >
      <input type="checkbox" checked={checked} disabled={!item.removable} readOnly className="mt-1" />
      <div className="min-w-0 flex-1">
        <div className="mono break-all">{item.path}</div>
        <div style={{ color: 'var(--win-text-secondary)' }}>
          {item.removable ? orDash(meta) : `${t('residue.locked')} · ${item.reasonIfLocked ?? ''}`}
        </div>
      </div>
    </li>
  );
}
