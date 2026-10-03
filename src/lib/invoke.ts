import { invoke } from '@tauri-apps/api/core';
import type { AppErrorPayload } from '@/types';

/**
 * Normalises anything thrown by `invoke` into an `AppErrorPayload`.
 *
 * Tauri serialises a command's `Err` value as-is, so a backend `AppError`
 * arrives as `{ code, message, detail? }`. Older builds (or a plain string
 * error) still need to produce something usable.
 */
export function toAppError(e: unknown): AppErrorPayload {
  if (typeof e === 'string') {
    return { code: 'UNKNOWN', message: e };
  }
  if (e && typeof e === 'object') {
    const obj = e as Record<string, unknown>;
    const message = typeof obj.message === 'string' ? obj.message : undefined;
    if (message) {
      return {
        code: (typeof obj.code === 'string' ? obj.code : 'UNKNOWN') as AppErrorPayload['code'],
        message,
        detail: typeof obj.detail === 'string' ? obj.detail : undefined,
      };
    }
  }
  return { code: 'UNKNOWN', message: String(e) };
}

/** Invokes a Tauri command and normalises any error it throws. */
export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}
