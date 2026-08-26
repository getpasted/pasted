import { Sliders, Command, Shield, Database, Cable, Blocks, Info, Bell, ScanSearch, LockKeyhole, History } from 'lucide-react';
import { translate } from '../localization/runtime';
import type { SettingsTab } from '../utils/appUiState';

export type { SettingsTab } from '../utils/appUiState';

interface SettingsTabsProps {
  activeTab: SettingsTab;
  onChange: (tab: SettingsTab) => void;
  showIntelligence?: boolean;
  showNotifications?: boolean;
  showSecurity?: boolean;
  showHotkeys?: boolean;
  showSearchHistory?: boolean;
}

const TABS = [
  { id: 'general', get label() { return translate('component.settingsTabs.general'); }, Icon: Sliders },
  { id: 'security', get label() { return translate('component.settingsTabs.security'); }, Icon: LockKeyhole },
  { id: 'functionality', get label() { return translate('component.settingsTabs.functionality'); }, Icon: Blocks },
  { id: 'search-history', get label() { return translate('component.settingsTabs.searchHistory'); }, Icon: History },
  { id: 'hotkeys', get label() { return translate('component.settingsTabs.hotkeys'); }, Icon: Command },
  { id: 'notifications', get label() { return translate('component.settingsTabs.notifications'); }, Icon: Bell },
  { id: 'app-exclusions', get label() { return translate('component.settingsTabs.appExclusions'); }, Icon: Shield },
  { id: 'storage', get label() { return translate('component.settingsTabs.storage'); }, Icon: Database },
  { id: 'analysis', get label() { return translate('component.settingsTabs.analysis'); }, Icon: ScanSearch },
  { id: 'intelligence', get label() { return translate('component.settingsTabs.intelligence'); }, Icon: Cable },
  { id: 'about', get label() { return translate('component.settingsTabs.about'); }, Icon: Info },
] as const;

export function SettingsTabs({ activeTab, onChange, showIntelligence = true, showNotifications = true, showSecurity = true, showHotkeys = true, showSearchHistory = true }: SettingsTabsProps) {
  return (
    <nav className="theme-surface settings-tabs ui-card-radius flex items-center gap-1 border p-1" aria-label={translate('component.settingsTabs.settingsSections')}>
      {TABS.filter(({ id }) => (
        (id !== 'intelligence' || showIntelligence)
        && (id !== 'notifications' || showNotifications)
        && (id !== 'security' || showSecurity)
        && (id !== 'hotkeys' || showHotkeys)
        && (id !== 'search-history' || showSearchHistory)
      )).map(({ id, label, Icon }) => (
        <button
          key={id}
          type="button"
          onClick={() => onChange(id)}
          aria-pressed={activeTab === id}
          title={label}
          className={`settings-tab ui-control-radius flex h-8 items-center justify-center gap-2 border border-transparent px-3 text-xs font-semibold transition-colors ${
            activeTab === id ? 'is-active' : ''
          }`}
        >
          <Icon className="w-3.5 h-3.5" />
          <span className="settings-tab-label">{label}</span>
        </button>
      ))}
    </nav>
  );
}
