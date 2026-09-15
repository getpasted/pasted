import type { HelpTopic, SettingsTab, TransformWorkspace } from './appUiStateCodec';

const SETTINGS_TABS = new Set<SettingsTab>([
  'general', 'functionality', 'search-history', 'hotkeys', 'notifications', 'security',
  'app-exclusions', 'storage', 'analysis', 'intelligence', 'about',
]);
const HELP_TOPICS = new Set<HelpTopic>([
  'getting-started', 'shortcuts-hud', 'privacy-capture', 'deletion-recovery',
  'analysis', 'transformations', 'cli',
]);
const TRANSFORM_WORKSPACES = new Set<TransformWorkspace>(['transforms', 'advanced', 'playground']);

export interface AppNavigationTarget {
  tab: string;
  settingsTab?: SettingsTab;
  helpTopic?: HelpTopic;
  transformWorkspace?: TransformWorkspace;
}

export function resolveAppNavigationTarget(
  route: string,
): AppNavigationTarget {
  const [tab, detail] = route.split(':', 2);
  if (tab === 'settings' && SETTINGS_TABS.has(detail as SettingsTab)) {
    return { tab, settingsTab: detail as SettingsTab };
  }
  if (tab === 'help' && HELP_TOPICS.has(detail as HelpTopic)) {
    return { tab, helpTopic: detail as HelpTopic };
  }
  if (tab === 'transformations' && TRANSFORM_WORKSPACES.has(detail as TransformWorkspace)) {
    return { tab, transformWorkspace: detail as TransformWorkspace };
  }
  return { tab };
}
