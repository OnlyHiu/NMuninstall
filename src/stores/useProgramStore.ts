import { create } from 'zustand';
import type {
  AppErrorPayload,
  ProgramInfo,
  ResidueReport,
  ScanStats,
  SortField,
  SortOrder,
  UninstallResult,
} from '@/types';
import { call, toAppError } from '@/lib/invoke';
import { useSettingsStore } from './useSettingsStore';

interface ProgramState {
  programs: ProgramInfo[];
  stats: ScanStats | null;
  elapsedMs: number;
  loading: boolean;
  error: AppErrorPayload | null;
  searchQuery: string;
  sortField: SortField;
  sortOrder: SortOrder;
  selectedId: string | null;
  detailOpen: boolean;
  /** Id of the program whose uninstaller is currently running. */
  busyId: string | null;
  lastResult: UninstallResult | null;

  load: () => Promise<void>;
  setSearchQuery: (q: string) => void;
  setSort: (field: SortField) => void;
  select: (id: string | null) => void;
  setDetailOpen: (open: boolean) => void;
  uninstall: (id: string, quiet: boolean, checkResidue: boolean) => Promise<UninstallResult>;
  removeProgram: (id: string) => void;
  checkResidue: (id: string) => Promise<ResidueReport>;
  clearResult: () => void;
  clearError: () => void;
}

let inFlight: Promise<void> | null = null;

export const useProgramStore = create<ProgramState>((set, get) => ({
  programs: [],
  stats: null,
  elapsedMs: 0,
  loading: false,
  error: null,
  searchQuery: '',
  sortField: 'displayName',
  sortOrder: 'asc',
  selectedId: null,
  detailOpen: false,
  busyId: null,
  lastResult: null,

  load: async () => {
    // A second call while a scan is running joins the first one.
    if (inFlight) return inFlight;

    const { showSystemComponents, show32BitPrograms } = useSettingsStore.getState().settings;

    set({ loading: true, error: null });
    const task = (async () => {
      try {
        const res = await call<{
          programs: ProgramInfo[];
          elapsedMs: number;
          stats: ScanStats;
        }>('list_programs', {
          includeSystem: showSystemComponents,
          includeX86: show32BitPrograms,
        });
        set({
          programs: res.programs,
          stats: res.stats,
          elapsedMs: res.elapsedMs,
          loading: false,
          error: null,
        });
        // Drop a selection that the new scan no longer contains.
        const selected = get().selectedId;
        if (selected && !res.programs.some((p) => p.id === selected)) {
          set({ selectedId: null, detailOpen: false });
        }
      } catch (e) {
        set({ loading: false, error: toAppError(e) });
      } finally {
        inFlight = null;
      }
    })();
    inFlight = task;
    return task;
  },

  setSearchQuery: (searchQuery) => set({ searchQuery }),

  setSort: (field) =>
    set((s) => {
      if (s.sortField !== field) {
        // Largest-first is the useful default for size; text reads better A→Z.
        return { sortField: field, sortOrder: field === 'estimatedSize' ? 'desc' : 'asc' };
      }
      return { sortOrder: s.sortOrder === 'asc' ? 'desc' : 'asc' };
    }),

  select: (selectedId) => set({ selectedId }),
  setDetailOpen: (detailOpen) => set({ detailOpen }),

  uninstall: async (id, quiet, checkResidue) => {
    set({ busyId: id, error: null });
    try {
      const result = await call<UninstallResult>('uninstall_program', { id, quiet, checkResidue });
      // The row leaves the list; a refresh brings it back if it is still there.
      set((s) => ({
        programs: s.programs.filter((p) => p.id !== id),
        lastResult: result,
        busyId: null,
        selectedId: s.selectedId === id ? null : s.selectedId,
      }));
      return result;
    } catch (e) {
      set({ busyId: null, error: toAppError(e) });
      throw e;
    }
  },

  removeProgram: (id) =>
    set((s) => ({
      programs: s.programs.filter((p) => p.id !== id),
      selectedId: s.selectedId === id ? null : s.selectedId,
      detailOpen: s.selectedId === id ? false : s.detailOpen,
    })),

  checkResidue: async (id) => call<ResidueReport>('check_residue', { id }),

  clearResult: () => set({ lastResult: null }),
  clearError: () => set({ error: null }),
}));
