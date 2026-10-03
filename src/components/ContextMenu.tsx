import { useEffect, useLayoutEffect, useRef, useState } from 'react';

export interface ContextMenuItem {
  id: string;
  label: string;
  disabled?: boolean;
  danger?: boolean;
  /** Rendered as a separator instead of a button. */
  separator?: boolean;
  hint?: string;
  onSelect?: () => void;
}

interface ContextMenuProps {
  items: ContextMenuItem[];
  x: number;
  y: number;
  onClose: () => void;
}

const MENU_WIDTH = 220;
const ITEM_HEIGHT = 32;
const MENU_PADDING = 4;

/** Right-click flyout with edge flipping and arrow-key navigation (§9.9). */
export function ContextMenu({ items, x, y, onClose }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });
  const [activeIndex, setActiveIndex] = useState(-1);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    let left = x;
    let top = y;
    if (left + rect.width > window.innerWidth - 8) left = Math.max(8, x - rect.width);
    if (top + rect.height > window.innerHeight - 8) {
      top = Math.max(8, window.innerHeight - rect.height - 8);
    }
    setPos({ left, top });
  }, [x, y]);

  useEffect(() => {
    ref.current?.focus();
  }, []);

  useEffect(() => {
    const onPointerDown = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const onScroll = () => onClose();
    document.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('resize', onScroll);
    window.addEventListener('blur', onScroll);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true);
      window.removeEventListener('resize', onScroll);
      window.removeEventListener('blur', onScroll);
    };
  }, [onClose]);

  const selectable = items
    .map((it, i) => ({ it, i }))
    .filter(({ it }) => !it.separator && !it.disabled);

  const move = (delta: number) => {
    if (selectable.length === 0) return;
    const current = selectable.findIndex((s) => s.i === activeIndex);
    const next = (current + delta + selectable.length) % selectable.length;
    setActiveIndex(selectable[next].i);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        onClose();
        break;
      case 'ArrowDown':
        e.preventDefault();
        move(1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        move(-1);
        break;
      case 'Enter':
      case ' ': {
        e.preventDefault();
        const item = items[activeIndex];
        if (item && !item.disabled && !item.separator) {
          item.onSelect?.();
          onClose();
        }
        break;
      }
      default:
        break;
    }
  };

  return (
    <div
      ref={ref}
      role="menu"
      tabIndex={-1}
      onKeyDown={onKeyDown}
      className="flyout fixed z-[60] overflow-hidden py-1 outline-none"
      style={{
        left: pos.left,
        top: pos.top,
        minWidth: MENU_WIDTH,
        padding: MENU_PADDING,
        background: 'var(--win-surface)',
        border: '1px solid var(--win-border-strong)',
        borderRadius: 'var(--win-radius-card)',
        boxShadow: 'var(--win-shadow-flyout)',
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item, i) => {
        if (item.separator) {
          return (
            <div
              key={item.id}
              role="separator"
              className="my-1 h-px"
              style={{ background: 'var(--win-border)' }}
            />
          );
        }
        return (
          <button
            key={item.id}
            role="menuitem"
            type="button"
            disabled={item.disabled}
            onClick={() => {
              item.onSelect?.();
              onClose();
            }}
            onMouseEnter={() => setActiveIndex(i)}
            className="flex w-full items-center justify-between gap-3 rounded-[4px] px-2.5 text-left text-[13px] disabled:opacity-40"
            style={{
              height: ITEM_HEIGHT,
              background:
                i === activeIndex && !item.disabled
                  ? 'color-mix(in srgb, var(--win-accent) 12%, transparent)'
                  : 'transparent',
              color: item.danger ? 'var(--win-danger)' : 'inherit',
            }}
          >
            <span className="truncate">{item.label}</span>
            {item.hint && (
              <span className="shrink-0 text-[11px] opacity-60 tabular">{item.hint}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}
