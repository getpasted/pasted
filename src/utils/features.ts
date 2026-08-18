import type { AppSettings } from '../types';

export type FeatureId =
  | 'analytics'
  | 'bins'
  | 'clipTypes'
  | 'contentClassification'
  | 'notes'
  | 'notifications'
  | 'appLock'
  | 'ocr'
  | 'transcriptions'
  | 'pinning'
  | 'protection'
  | 'queue'
  | 'revisions'
  | 'hud'
  | 'trash'
  | 'transformations'
  | 'activityLog'
  | 'types'
  | 'sources'
  | 'cli'
  | 'help';

export type FeatureSettingKey =
  | 'enableAnalytics'
  | 'enableBins'
  | 'enableClipTypes'
  | 'enableContentClassification'
  | 'enableNotes'
  | 'enableNotifications'
  | 'enableAppLock'
  | 'enableOcr'
  | 'enableTranscriptions'
  | 'enablePinning'
  | 'enableProtection'
  | 'enableQueue'
  | 'enableRevisions'
  | 'enableHud'
  | 'enableTrash'
  | 'enableTransformations'
  | 'enableActivityLog'
  | 'enableTypes'
  | 'enableSources'
  | 'enableCli'
  | 'enableHelp';

export type FeatureGroupId = 'library' | 'discovery' | 'workflow' | 'app';

export interface FeatureGroupDefinition {
  id: FeatureGroupId;
  label: string;
  description: string;
}

export const FEATURE_GROUPS: readonly FeatureGroupDefinition[] = [
  { id: 'library', label: 'Library', description: 'Organize, preserve, and manage clipboard history.' },
  { id: 'discovery', label: 'Intelligence and discovery', description: 'Understand clip contents and browse useful collections.' },
  { id: 'workflow', label: 'Workflow Tools', description: 'Use clips faster and build more capable workflows.' },
  { id: 'app', label: 'App and support', description: 'Control feedback and access supporting tools.' },
] as const;

export interface FeatureDefinition {
  id: FeatureId;
  group: FeatureGroupId;
  settingKey: FeatureSettingKey;
  label: string;
  description: string;
  simple: boolean;
  caution?: string;
}

export const FEATURE_DEFINITIONS: readonly FeatureDefinition[] = [
  { id: 'bins', group: 'library', settingKey: 'enableBins', label: 'Bins', description: 'Organize clips manually or automatically with Smart Bins.', simple: false },
  { id: 'clipTypes', group: 'library', settingKey: 'enableClipTypes', label: 'Clip Types', description: 'Show structural Clip Types and their collections.', simple: false },
  { id: 'contentClassification', group: 'discovery', settingKey: 'enableContentClassification', label: 'Content Classification', description: 'Assign registered Content Types to analyzable text.', simple: true },
  { id: 'notes', group: 'library', settingKey: 'enableNotes', label: 'Notes', description: 'Annotate clips and browse the Noted collection.', simple: false },
  { id: 'notifications', group: 'app', settingKey: 'enableNotifications', label: 'Notifications', description: 'Show interactive capture feedback without interrupting the current workflow.', simple: false },
  {
    id: 'appLock',
    group: 'app',
    settingKey: 'enableAppLock',
    label: 'App Lock',
    description: 'Require authentication before showing clipboard history.',
    simple: false,
    caution: 'The saved passphrase and lock preferences remain available when App Lock is re-enabled.',
  },
  { id: 'ocr', group: 'discovery', settingKey: 'enableOcr', label: 'OCR', description: 'Automatically extract searchable text from copied images.', simple: false },
  { id: 'transcriptions', group: 'discovery', settingKey: 'enableTranscriptions', label: 'Transcriptions', description: 'Create searchable text from copied audio files.', simple: false },
  { id: 'pinning', group: 'library', settingKey: 'enablePinning', label: 'Pinning', description: 'Keep important clips at the top of history.', simple: false },
  {
    id: 'protection',
    group: 'library',
    settingKey: 'enableProtection',
    label: 'Protection',
    description: 'Protect clips from deletion and automatic retention.',
    simple: false,
    caution: 'Previously protected clips remain protected. Re-enable this feature to change them.',
  },
  { id: 'queue', group: 'workflow', settingKey: 'enableQueue', label: 'Copy Queue', description: 'Collect copied text and paste it back in sequence.', simple: false },
  {
    id: 'revisions',
    group: 'library',
    settingKey: 'enableRevisions',
    label: 'Revision History',
    description: 'Keep restorable snapshots before clips change.',
    simple: true,
    caution: 'New edits and Transforms will not be reversible while Revision History is disabled.',
  },
  { id: 'hud', group: 'workflow', settingKey: 'enableHud', label: 'HUD', description: 'Open the compact keyboard-driven clipboard window.', simple: false },
  {
    id: 'trash',
    group: 'library',
    settingKey: 'enableTrash',
    label: 'Trash',
    description: 'Recover deleted clips before they are permanently removed.',
    simple: true,
    caution: 'When disabled, deleting a clip permanently removes it.',
  },
  { id: 'transformations', group: 'workflow', settingKey: 'enableTransformations', label: 'Transformations', description: 'Run text workflows and receive Smart Action suggestions.', simple: false },
  { id: 'analytics', group: 'discovery', settingKey: 'enableAnalytics', label: 'Insights', description: 'Browse active-library composition and recent additions.', simple: false },
  { id: 'activityLog', group: 'app', settingKey: 'enableActivityLog', label: 'Activity', description: 'Record and inspect important app events.', simple: false },
  { id: 'types', group: 'discovery', settingKey: 'enableTypes', label: 'Content Types', description: 'Show recognized Content Types and their collections.', simple: false },
  { id: 'sources', group: 'discovery', settingKey: 'enableSources', label: 'Sources', description: 'Show the applications associated with captured clips and their collections.', simple: false },
  { id: 'cli', group: 'app', settingKey: 'enableCli', label: 'Command-Line Interface', description: 'Use pasted to automate clipboard workflows.', simple: false },
  { id: 'help', group: 'app', settingKey: 'enableHelp', label: 'Help', description: 'Show in-app documentation and its navigation entry.', simple: true },
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
  if (route.startsWith('clip_type-')) return 'clipTypes';
  if (route.startsWith('type-')) return 'types';
  if (route.startsWith('source-')) return 'sources';
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
