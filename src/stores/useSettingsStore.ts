import { create } from 'zustand';
import type { AppErrorPayload, AppSettings } from '@/types';
import { DEFAULT_SETTINGS } from '@/types';
import { call, toAppError } from '@/lib/invoke';

interface SettingsState {
  settings: AppSettings;
  /** True once the backend has answered at least once. */
  ready: boolean;
  error: AppErrorPayload | null;
  savedAt: number | null;

  load: () => Promise<void>;
  update: (patch: Partial<AppSettings>) => Promise<void>;
  save: () => Promise<void>;
  clearError: () => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  ready: false,
  error: null,
  savedAt: null,

  load: async () => {
    try {
      const settings = await call<AppSettings>('get_settings');
      set({ settings, ready: true, error: null });
    } catch (e) {
      // Falling back to defaults keeps the app usable; surface the reason.
      set({ settings: DEFAULT_SETTINGS, ready: true, error: toAppError(e) });
    }
  },

  /** Optimistic local update; `save` reconciles with the backend's normalised value. */
  update: async (patch) => {
    set((s) => ({ settings: { ...s.settings, ...patch } }));
    await get().save();
  },

  save: async () => {
    try {
      // The backend clamps fields; its answer is authoritative.
      const settings = await call<AppSettings>('save_settings', { settings: get().settings });
      set({ settings, savedAt: Date.now(), error: null });
    } catch (e) {
      set({ error: toAppError(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
