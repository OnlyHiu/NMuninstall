import type { CSSProperties, ReactNode } from 'react';
import { useEffect, useRef } from 'react';
import { cn } from '@/lib/cn';

interface DialogProps {
  open: boolean;
  title: string;
  /** Set to false for destructive flows, where a stray backdrop click must not dismiss. */
  dismissOnBackdrop?: boolean;
  width?: number;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Modal dialog with a focus trap, `Esc` handling and initial focus placement —
 * the behaviour every flyout in the app relies on (§9.9).
 */
export function Dialog({
  open,
  title,
  dismissOnBackdrop = true,
  width = 460,
  onClose,
  children,
  footer,
}: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const restoreTo = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    restoreTo.current = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const first = panel?.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key !== 'Tab' || !panel) return;
      const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (el) => el.offsetParent !== null,
      );
      if (items.length === 0) return;
      const firstEl = items[0];
      const lastEl = items[items.length - 1];
      const active = document.activeElement as HTMLElement | null;
      if (e.shiftKey && (active === firstEl || !panel.contains(active))) {
        e.preventDefault();
        lastEl.focus();
      } else if (!e.shiftKey && active === lastEl) {
        e.preventDefault();
        firstEl.focus();
      }
    };

    document.addEventListener('keydown', onKeyDown, true);
    return () => {
      document.removeEventListener('keydown', onKeyDown, true);
      restoreTo.current?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      onMouseDown={(e) => {
        if (dismissOnBackdrop && e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="flyout max-h-[85vh] overflow-hidden rounded-lg shadow-[var(--win-shadow-flyout)]"
        style={
          {
            width,
            maxWidth: '100%',
            background: 'var(--win-surface)',
            border: '1px solid var(--win-border-strong)',
          } as CSSProperties
        }
      >
        <header
          className="flex items-center justify-between px-5 pt-4 pb-3"
          style={{ borderBottom: '1px solid var(--win-border)' }}
        >
          <h2 className="text-[15px] font-semibold">{title}</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="close"
            className={cn(
              'flex size-7 items-center justify-center rounded hover:bg-black/10',
              'dark:hover:bg-white/10',
            )}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <path
                d="M1 1l10 10M11 1L1 11"
                stroke="currentColor"
                strokeWidth="1.2"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </header>

        <div className="max-h-[60vh] overflow-y-auto px-5 py-4">{children}</div>

        {footer && (
          <footer
            className="flex items-center justify-end gap-2 px-5 py-3"
            style={{ borderTop: '1px solid var(--win-border)' }}
          >
            {footer}
          </footer>
        )}
      </div>
    </div>
  );
}
