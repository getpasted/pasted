import type { AppSettings } from '../types';

export type FeatureId =
  | 'analytics'
  | 'bins'
  | 'contentDetection'
  | 'diagnostics'
  | 'notes'
  | 'notifications'
  | 'ocr'
  | 'pinning'
  | 'protection'
  | 'queue'
  | 'revisions'
  | 'hud'
  | 'trash'
  | 'transformations'
  | 'activityLog'
  | 'cli'
  | 'help';

export type FeatureSettingKey =
  | 'enableAnalytics'
  | 'enableBins'
  | 'enableContentDetection'
  | 'enableDiagnostics'
  | 'enableNotes'
  | 'enableNotifications'
  | 'enableOcr'
  | 'enablePinning'
  | 'enableProtection'
  | 'enableQueue'
  | 'enableRevisions'
  | 'enableHud'
  | 'enableTrash'
  | 'enableTransformations'
  | 'enableActivityLog'
  | 'enableCli'
  | 'enableHelp';

export interface FeatureDefinition {
  id: FeatureId;
  settingKey: FeatureSettingKey;
  label: string;
  description: string;
  simple: boolean;
  caution?: string;
}

export const FEATURE_DEFINITIONS: readonly FeatureDefinition[] = [
  { id: 'bins', settingKey: 'enableBins', label: 'Bins', description: 'Organize clips manually or automatically with Smart Bins.', simple: false },
  { id: 'contentDetection', settingKey: 'enableContentDetection', label: 'Content Detection', description: 'Classify new text clips for Smart Bins and search.', simple: true },
  { id: 'diagnostics', settingKey: 'enableDiagnostics', label: 'Diagnostics', description: 'Inspect background work, health, and developer tools.', simple: false },
  { id: 'notes', settingKey: 'enableNotes', label: 'Notes', description: 'Annotate clips and browse the Noted collection.', simple: false },
  { id: 'notifications', settingKey: 'enableNotifications', label: 'Notifications', description: 'Show interactive capture feedback without interrupting your work.', simple: false },
  { id: 'ocr', settingKey: 'enableOcr', label: 'OCR', description: 'Extract searchable text from copied images.', simple: false },
  { id: 'pinning', settingKey: 'enablePinning', label: 'Pinning', description: 'Keep important clips at the top of history.', simple: false },
  {
    id: 'protection',
    settingKey: 'enableProtection',
    label: 'Protection',
    description: 'Protect clips from deletion and automatic retention.',
    simple: false,
    caution: 'Previously protected clips remain protected. Re-enable this feature to change them.',
  },
  { id: 'queue', settingKey: 'enableQueue', label: 'Copy Queue', description: 'Collect copied text and paste it back in sequence.', simple: false },
  {
    id: 'revisions',
    settingKey: 'enableRevisions',
    label: 'Revision History',
    description: 'Keep restorable snapshots before clips change.',
    simple: true,
    caution: 'New edits and Transforms will not be reversible while Revision History is disabled.',
  },
  { id: 'hud', settingKey: 'enableHud', label: 'Quick HUD', description: 'Open the compact keyboard-driven clipboard window.', simple: false },
  {
    id: 'trash',
    settingKey: 'enableTrash',
    label: 'Trash',
    description: 'Recover deleted clips before they are permanently removed.',
    simple: true,
    caution: 'When disabled, deleting a clip permanently removes it.',
  },
  { id: 'transformations', settingKey: 'enableTransformations', label: 'Transformations', description: 'Run saved, advanced, and AI-assisted text workflows.', simple: false },
  { id: 'analytics', settingKey: 'enableAnalytics', label: 'Analytics & Insights', description: 'Explore clipboard usage and content trends.', simple: false },
  { id: 'activityLog', settingKey: 'enableActivityLog', label: 'Activity Log', description: 'Record and inspect important Pasted events.', simple: false },
  { id: 'cli', settingKey: 'enableCli', label: 'Command-Line Interface', description: 'Use pasted to automate clipboard workflows.', simple: false },
  { id: 'help', settingKey: 'enableHelp', label: 'Help & Documentation', description: 'Show in-app documentation and its navigation entry.', simple: true },
] as const;

export const FEATURE_SETTING_KEYS = FEATURE_DEFINITIONS.map(({ settingKey }) => settingKey);

export type FeaturePreset = 'full' | 'simple' | 'custom';

export function isFeatureEnabled(settings: AppSettings, featureId: FeatureId): boolean {
  const definition = FEATURE_DEFINITIONS.find(({ id }) => id === featureId);
  return definition ? settings[definition.settingKey] : false;
}

export function featureUpdatesForPreset(preset: Exclude<FeaturePreset, 'custom'>): Partial<AppSettings> {
  return Object.fromEntries(
    FEATURE_DEFINITIONS.map(({ settingKey, simple }) => [settingKey, preset === 'full' || simple]),
  ) as Partial<AppSettings>;
}

export function activeFeaturePreset(settings: AppSettings): FeaturePreset {
  const isFull = FEATURE_DEFINITIONS.every(({ settingKey }) => settings[settingKey]);
  if (isFull) return 'full';
  const isSimple = FEATURE_DEFINITIONS.every(({ settingKey, simple }) => settings[settingKey] === simple);
  return isSimple ? 'simple' : 'custom';
}

export function enabledFeatureRecord(settings: AppSettings): Record<FeatureId, boolean> {
  return Object.fromEntries(
    FEATURE_DEFINITIONS.map(({ id }) => [id, isFeatureEnabled(settings, id)]),
  ) as Record<FeatureId, boolean>;
}

export function featureForRoute(route: string): FeatureId | null {
  const tab = route.split(':', 1)[0];
  const routes: Record<string, FeatureId> = {
    sequential: 'queue',
    pinned: 'pinning',
    protected: 'protection',
    notes: 'notes',
    trash: 'trash',
    bin: 'bins',
    analytics: 'analytics',
    transformations: 'transformations',
    activity: 'activityLog',
    help: 'help',
  };
  return routes[tab] ?? null;
}
