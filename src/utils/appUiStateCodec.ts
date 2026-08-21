export const SIDEBAR_SECTION_IDS = ['clips', 'bins', 'clipTypes', 'types', 'fileFormats', 'sources', 'tools'] as const;

export type SidebarSectionId = typeof SIDEBAR_SECTION_IDS[number];
export type SidebarSectionState = Record<SidebarSectionId, boolean>;

export const SETTINGS_TABS = ['general', 'functionality', 'hotkeys', 'notifications', 'security', 'app-exclusions', 'storage', 'analysis', 'intelligence', 'about'] as const;
export const HELP_TOPICS = ['getting-started', 'cli', 'shortcuts-hud', 'privacy-capture', 'deletion-recovery', 'analysis', 'transformations'] as const;
export const TRANSFORM_WORKSPACES = ['transforms', 'advanced', 'playground'] as const;

export type SettingsTab = typeof SETTINGS_TABS[number];
export type HelpTopic = typeof HELP_TOPICS[number];
export type TransformWorkspace = typeof TRANSFORM_WORKSPACES[number];

export interface AppUiState {
  version: 2;
  currentTab: string;
  settingsTab: SettingsTab;
  helpTopic: HelpTopic;
  transformWorkspace: TransformWorkspace;
  selectedBinId: number | null;
  selectedClipId: number | null;
  isSidebarCollapsed: boolean;
  sidebarSections: SidebarSectionState;
}

export const DEFAULT_SIDEBAR_SECTIONS: SidebarSectionState = {
  clips: true,
  bins: true,
  clipTypes: true,
  fileFormats: true,
  types: true,
  sources: true,
  tools: true,
};

export const DEFAULT_APP_UI_STATE: AppUiState = {
  version: 2,
  currentTab: 'all',
  settingsTab: 'general',
  helpTopic: 'getting-started',
  transformWorkspace: 'transforms',
  selectedBinId: null,
  selectedClipId: null,
  isSidebarCollapsed: false,
  sidebarSections: DEFAULT_SIDEBAR_SECTIONS,
};

const STANDARD_TABS = new Set([
  'all', 'search', 'sequential', 'pinned', 'protected', 'concealed', 'notes', 'trash', 'bin',
  'analytics', 'transformations', 'activity', 'help', 'settings',
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
  if (!value.startsWith('clip_type-') && !value.startsWith('content_type-') && !value.startsWith('file_format-') && !value.startsWith('source-')) return false;
  const encodedValue = value.slice(value.indexOf('-') + 1);
  if (!encodedValue) return false;
  try {
    decodeURIComponent(encodedValue);
    return true;
  } catch {
    return false;
  }
}

function oneOf<const T extends readonly string[]>(value: unknown, values: T, fallback: T[number]): T[number] {
  return typeof value === 'string' && (values as readonly string[]).includes(value) ? value as T[number] : fallback;
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
    version: 2,
    currentTab: currentTab === 'bin' && selectedBinId === null ? 'all' : currentTab,
    settingsTab: oneOf(record.settingsTab, SETTINGS_TABS, 'general'),
    helpTopic: oneOf(record.helpTopic, HELP_TOPICS, 'getting-started'),
    transformWorkspace: oneOf(record.transformWorkspace, TRANSFORM_WORKSPACES, 'transforms'),
    selectedBinId,
    selectedClipId: clipIdentifier(record.selectedClipId),
    isSidebarCollapsed: record.isSidebarCollapsed === true,
    sidebarSections,
  };
}
