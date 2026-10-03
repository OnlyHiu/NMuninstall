import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { ProgramInfo } from '@/types';

interface ProgramIconProps {
  program: ProgramInfo;
  size?: number;
}

/**
 * Renders the program's `DisplayIcon` through the asset protocol.
 *
 * WebView2 handles `.ico` and PE resources natively. Any load failure swaps in
 * a neutral glyph so the list never shows a broken-image box.
 */
export function ProgramIcon({ program, size = 28 }: ProgramIconProps) {
  const [failed, setFailed] = useState(false);
  const src = program.iconPath ? convertFileSrc(program.iconPath) : undefined;
  const show = Boolean(src) && !failed;

  return (
    <span
      className="flex shrink-0 items-center justify-center"
      style={{ width: size, height: size }}
      aria-hidden="true"
    >
      {show ? (
        <img
          src={src}
          alt=""
          width={size}
          height={size}
          loading="lazy"
          decoding="async"
          style={{ width: size, height: size }}
          className="object-contain"
          onError={() => setFailed(true)}
        />
      ) : (
        <span
          className="flex items-center justify-center rounded-[4px] text-[13px] font-semibold uppercase"
          style={{
            width: size,
            height: size,
            background: 'color-mix(in srgb, var(--win-accent) 14%, transparent)',
            color: 'var(--win-accent)',
          }}
        >
          {program.displayName.trim().charAt(0) || '?'}
        </span>
      )}
    </span>
  );
}
