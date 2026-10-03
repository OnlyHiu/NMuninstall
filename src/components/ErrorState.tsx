import type { AppErrorPayload } from '@/types';
import { useT } from '@/i18n/useT';
import { Button } from './Button';

/** Full-panel failure state for a scan that could not complete (F-605). */
export function ErrorState({ error, onRetry }: { error: AppErrorPayload; onRetry: () => void }) {
  const t = useT();
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
      <svg width="34" height="34" viewBox="0 0 24 24" aria-hidden="true" style={{ color: 'var(--win-danger)' }}>
        <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M12 7v6" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
        <circle cx="12" cy="16.4" r="1" fill="currentColor" />
      </svg>
      <p className="text-[14px] font-medium">{t('error.loadFailed')}</p>
      <p className="max-w-md text-[13px]" style={{ color: 'var(--win-text-secondary)' }}>
        {error.message}
      </p>
      {error.detail && (
        <p className="mono max-w-md break-all text-[11px] opacity-70">{error.detail}</p>
      )}
      <div className="mt-1 flex gap-2">
        <Button variant="accent" onClick={onRetry}>
          {t('error.retry')}
        </Button>
        {error.detail && (
          <Button
            onClick={() => void navigator.clipboard.writeText(error.detail ?? '')}
          >
            {t('error.copyDetail')}
          </Button>
        )}
      </div>
    </div>
  );
}
