import { scheduleBackupClientStatePersistence } from './backupClientState';

export const APP_UI_STATE_KEY = 'pasted_app_ui_state';

export const SIDEBAR_SECTION_IDS = ['clips', 'bins', 'types', 'sources', 'tools'] as const;

export type SidebarSectionId = typeof SIDEBAR_SECTION_IDS[number];
export type SidebarSectionState = Record<SidebarSectionId, boolean>;

export interface AppUiState {
  version: 1;
  currentTab: string;
  selectedBinId: number | null;
  selectedClipId: number | null;
  isSidebarCollapsed: boolean;
  sidebarSections: SidebarSectionState;
}

export const DEFAULT_SIDEBAR_SECTIONS: SidebarSectionState = {
  clips: true,
  bins: true,
  types: true,
  sources: true,
  tools: true,
};

export const DEFAULT_APP_UI_STATE: AppUiState = {
  version: 1,
  currentTab: 'all',
  selectedBinId: null,
  selectedClipId: null,
  isSidebarCollapsed: false,
  sidebarSections: DEFAULT_SIDEBAR_SECTIONS,
};

const STANDARD_TABS = new Set([
  'all',
  'search',
  'sequential',
  'pinned',
  'protected',
  'notes',
  'trash',
  'bin',
  'analytics',
  'transformations',
  'activity',
  'help',
  'settings',
]);

function positiveInteger(value: unknown): number | null {
  return typeof value === 'number' && Number.isSafeInteger(value) && value > 0 ? value : null;
}

function clipIdentifier(value: unknown): number | null {
  return typeof value === 'number' && Number.isSafeInteger(value) && value !== 0 ? value : null;
}

function validTab(value: unknown): value is string {
  if (typeof value !== 'string' || value.length > 512) return false;
  if (STANDARD_TABS.has(value)) return true;
  if (!value.startsWith('type-') && !value.startsWith('source-')) return false;
  const encodedValue = value.slice(value.indexOf('-') + 1);
  if (!encodedValue) return false;
  try {
    decodeURIComponent(encodedValue);
    return true;
  } catch {
    return false;
  }
}

export function parseAppUiState(value: unknown): AppUiState {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return DEFAULT_APP_UI_STATE;
  const record = value as Record<string, unknown>;
  const currentTab = validTab(record.currentTab) ? record.currentTab : 'all';
  const selectedBinId = currentTab === 'bin' ? positiveInteger(record.selectedBinId) : null;
  const rawSections = record.sidebarSections && typeof record.sidebarSections === 'object'
    && !Array.isArray(record.sidebarSections)
    ? record.sidebarSections as Record<string, unknown>
    : {};
  const sidebarSections = { ...DEFAULT_SIDEBAR_SECTIONS };
  for (const id of SIDEBAR_SECTION_IDS) {
    if (typeof rawSections[id] === 'boolean') sidebarSections[id] = rawSections[id];
  }

  return {
    version: 1,
    currentTab: currentTab === 'bin' && selectedBinId === null ? 'all' : currentTab,
    selectedBinId,
    // Queue rows use stable negative IDs derived from their persisted item IDs.
    selectedClipId: clipIdentifier(record.selectedClipId),
    isSidebarCollapsed: record.isSidebarCollapsed === true,
    sidebarSections,
  };
}

export function readAppUiState(): AppUiState {
  try {
    const saved = localStorage.getItem(APP_UI_STATE_KEY);
    return saved ? parseAppUiState(JSON.parse(saved)) : DEFAULT_APP_UI_STATE;
  } catch {
    return DEFAULT_APP_UI_STATE;
  }
}

export function writeAppUiState(state: AppUiState) {
  try {
    localStorage.setItem(APP_UI_STATE_KEY, JSON.stringify(state));
    scheduleBackupClientStatePersistence();
  } catch {
    // UI state is best-effort; the database and clipboard library remain authoritative.
  }
}
