import { Sliders, Command, Shield, Database, Cable, Blocks, Info, Bell, ScanSearch } from 'lucide-react';

export type SettingsTab = 'general' | 'functionality' | 'hotkeys' | 'notifications' | 'app-exclusions' | 'storage' | 'analysis' | 'intelligence' | 'about';

interface SettingsTabsProps {
  activeTab: SettingsTab;
  onChange: (tab: SettingsTab) => void;
  showIntelligence?: boolean;
  showNotifications?: boolean;
  showAnalysis?: boolean;
}

const TABS = [
  { id: 'general', label: 'General', Icon: Sliders },
  { id: 'functionality', label: 'Functionality', Icon: Blocks },
  { id: 'hotkeys', label: 'Hotkeys', Icon: Command },
  { id: 'notifications', label: 'Notifications', Icon: Bell },
  { id: 'app-exclusions', label: 'App Exclusions', Icon: Shield },
  { id: 'storage', label: 'Storage', Icon: Database },
  { id: 'analysis', label: 'Analysis', Icon: ScanSearch },
  { id: 'intelligence', label: 'Intelligence', Icon: Cable },
  { id: 'about', label: 'About', Icon: Info },
] as const;

export function SettingsTabs({ activeTab, onChange, showIntelligence = true, showNotifications = true, showAnalysis = true }: SettingsTabsProps) {
  return (
    <nav className="theme-surface settings-tabs flex items-center gap-1 rounded-xl border p-1" aria-label="Settings sections">
      {TABS.filter(({ id }) => (
        (id !== 'intelligence' || showIntelligence)
        && (id !== 'analysis' || showAnalysis)
        && (id !== 'notifications' || showNotifications)
      )).map(({ id, label, Icon }) => (
        <button
          key={id}
          type="button"
          onClick={() => onChange(id)}
          aria-pressed={activeTab === id}
          title={label}
          className={`settings-tab flex h-8 items-center justify-center gap-2 rounded-lg border border-transparent px-3 text-xs font-semibold transition-colors ${
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
