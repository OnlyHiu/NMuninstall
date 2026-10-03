import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { call } from '@/lib/invoke';
import type { ProgramInfo } from '@/types';

interface ProgramIconProps {
  program: ProgramInfo;
  size?: number;
}

/**
 * Files WebView2 can decode on its own.
 *
 * This is the exception, not the rule: `DisplayIcon` overwhelmingly points at
 * an `.exe` or `.dll`, which is a PE container and not an image at all.
 */
const NATIVELY_DECODABLE = /\.(ico|png|bmp|gif|jpe?g|webp|svg)$/i;

interface Render {
  /** The program this result belongs to, so recycled rows can discard it. */
  id: string;
  dataUrl: string | null;
  /** The URL did load as an `<img>`; stop using it. */
  broken: boolean;
}

/**
 * Renders a program's icon.
 *
 * The icon is *not* a URL the browser can fetch. `iconPath` is a PE or ICO
 * container, and handing those raw bytes to `<img src>` made every single icon
 * fail and collapse into a letter avatar. The backend now asks the Windows
 * shell for the icon and returns a PNG data URL instead.
 */
export function ProgramIcon({ program, size = 28 }: ProgramIconProps) {
  const { id, iconPath, displayName } = program;
  const [render, setRender] = useState<Render | null>(null);

  useEffect(() => {
    if (!iconPath) return;
    let cancelled = false;
    call<string | null>('get_program_icon', { id })
      .then((dataUrl) => {
        if (!cancelled) setRender({ id, dataUrl, broken: false });
      })
      .catch(() => {
        // A failed render only means "no icon". It must never surface as an
        // error, and the row must still fall back to its letter.
        if (!cancelled) setRender({ id, dataUrl: null, broken: false });
      });
    return () => {
      cancelled = true;
    };
  }, [id, iconPath]);

  // Virtualised rows are recycled, so this state can still belong to whichever
  // program occupied the slot a moment ago. Keying on the id lets us drop it
  // without a `setState` inside the effect.
  const mine = render?.id === id ? render : undefined;
  const fallback =
    iconPath && NATIVELY_DECODABLE.test(iconPath) ? convertFileSrc(iconPath) : null;
  const src = mine && !mine.broken ? (mine.dataUrl ?? fallback) : null;
  // Hold the slot empty until the backend answers; swapping a letter avatar in
  // and then out again reads as a flicker on every scroll.
  const settled = Boolean(mine) || !iconPath;

  return (
    <span
      className="flex shrink-0 items-center justify-center"
      style={{ width: size, height: size }}
      aria-hidden="true"
    >
      {src ? (
        <img
          src={src}
          alt=""
          width={size}
          height={size}
          decoding="async"
          style={{ width: size, height: size }}
          className="object-contain"
          onError={() =>
            setRender((prev) => (prev?.id === id ? { ...prev, broken: true } : prev))
          }
        />
      ) : settled ? (
        <span
          className="flex items-center justify-center rounded-[4px] text-[13px] font-semibold uppercase"
          style={{
            width: size,
            height: size,
            background: 'color-mix(in srgb, var(--win-accent) 14%, transparent)',
            color: 'var(--win-accent)',
          }}
        >
          {displayName.trim().charAt(0) || '?'}
        </span>
      ) : null}
    </span>
  );
}
