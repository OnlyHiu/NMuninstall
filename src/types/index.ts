/** Mirrors `src-tauri/src/models.rs` — keep the two in sync. */

export type Hive = 'HKLM' | 'HKCU';
export type RegistryView = '64' | '32';
export type UninstallStatus = 'completed' | 'started' | 'failed';
export type Theme = 'system' | 'light' | 'dark';
export type Language = 'zh-CN' | 'en-US';
export type ListDensity = 'comfortable' | 'compact';
export type SortField =
  | 'displayName'
  | 'displayVersion'
  | 'publisher'
  | 'installDate'
  | 'estimatedSize';
export type SortOrder = 'asc' | 'desc';
export type ResidueKind = 'registry' | 'directory';

export interface ProgramInfo {
  id: string;
  keyName: string;
  regPath: string;
  displayName: string;
  displayVersion?: string;
  publisher?: string;
  /** Normalised to `yyyy-MM-dd` when the raw value was 8 digits. */
  installDate?: string;
  rawInstallDate?: string;
  installLocation?: string;
  /** Bytes. */
  estimatedSize?: number;
  uninstallString?: string;
  quietUninstallString?: string;
  displayIcon?: string;
  /** Resolved absolute path; present only when the file exists. */
  iconPath?: string;
  systemComponent: boolean;
  windowsInstaller: boolean;
  parentKeyName?: string;
  noModify: boolean;
  noRepair: boolean;
  isSystem: boolean;
  hive: Hive;
  view: RegistryView;
  is64Bit: boolean;
  canUninstall: boolean;
}

export interface ScanStats {
  total: number;
  skippedNoName: number;
  skippedInaccessible: number;
  deduplicated: number;
  scannedKeys: number;
}

export interface ProgramsResponse {
  programs: ProgramInfo[];
  elapsedMs: number;
  stats: ScanStats;
}

export interface ResidueItem {
  id: string;
  kind: ResidueKind;
  path: string;
  sizeBytes?: number;
  fileCount?: number;
  removable: boolean;
  reasonIfLocked?: string;
}

export interface ResidueReport {
  registryKeys: ResidueItem[];
  installDirs: ResidueItem[];
  cleanableCount: number;
}

export interface UninstallResult {
  status: UninstallStatus;
  message: string;
  executable: string;
  args: string[];
  exitCode?: number;
  waited: boolean;
  durationMs: number;
  quietFallback: boolean;
  residue?: ResidueReport;
}

export interface CleanFailure {
  path: string;
  error: string;
}

export interface CleanResult {
  removed: string[];
  failed: CleanFailure[];
}

export interface AppSettings {
  showSystemComponents: boolean;
  show32BitPrograms: boolean;
  theme: Theme;
  confirmBeforeUninstall: boolean;
  checkResidueAfterUninstall: boolean;
  language: Language;
  listDensity: ListDensity;
  defaultQuietUninstall: boolean;
  uninstallTimeoutSecs: number;
  /** `true` → the close button hides to the tray; `false` → it exits. */
  closeToTray: boolean;
}

export interface AppInfo {
  version: string;
  tauriVersion: string;
  os: string;
  arch: string;
  logDir: string;
  configPath: string;
}

export type AppErrorCode =
  | 'NOT_FOUND'
  | 'NO_UNINSTALL_STRING'
  | 'PARSE_FAILED'
  | 'SPAWN_FAILED'
  | 'UNINSTALL_TIMEOUT'
  | 'REGISTRY_READ_FAILED'
  | 'PATH_NOT_ALLOWED'
  | 'PATH_NOT_FOUND'
  | 'CLEAN_FAILED'
  | 'SETTINGS_READ_FAILED'
  | 'SETTINGS_WRITE_FAILED'
  | 'NOT_SUPPORTED'
  | 'INTERNAL';

export interface AppErrorPayload {
  code: AppErrorCode | 'UNKNOWN';
  message: string;
  detail?: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  showSystemComponents: false,
  show32BitPrograms: true,
  theme: 'system',
  confirmBeforeUninstall: true,
  checkResidueAfterUninstall: false,
  language: 'zh-CN',
  listDensity: 'comfortable',
  defaultQuietUninstall: false,
  uninstallTimeoutSecs: 300,
  closeToTray: false,
};
