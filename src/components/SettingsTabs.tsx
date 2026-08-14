import { Sliders, Command, Shield, Database, Cable, Blocks, Info, Bell, ScanSearch } from 'lucide-react';

export type SettingsTab = 'general' | 'features' | 'analysis' | 'notifications' | 'hotkeys' | 'connections' | 'blacklist' | 'storage' | 'about';

interface SettingsTabsProps {
  activeTab: SettingsTab;
  onChange: (tab: SettingsTab) => void;
  showConnections?: boolean;
  showNotifications?: boolean;
  showDetection?: boolean;
}

const TABS = [
  { id: 'general', label: 'General', Icon: Sliders },
  { id: 'features', label: 'Functionality', Icon: Blocks },
  { id: 'analysis', label: 'Analysis', Icon: ScanSearch },
  { id: 'notifications', label: 'Notifications', Icon: Bell },
  { id: 'hotkeys', label: 'Hotkeys', Icon: Command },
  { id: 'connections', label: 'Connections', Icon: Cable },
  { id: 'blacklist', label: 'Blacklist', Icon: Shield },
  { id: 'storage', label: 'Storage', Icon: Database },
  { id: 'about', label: 'About', Icon: Info },
] as const;

export function SettingsTabs({ activeTab, onChange, showConnections = true, showNotifications = true, showDetection = true }: SettingsTabsProps) {
  return (
    <nav className="theme-surface settings-tabs flex items-center gap-1 rounded-xl border p-1" aria-label="Settings sections">
      {TABS.filter(({ id }) => (
        (id !== 'connections' || showConnections)
        && (id !== 'analysis' || showDetection)
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
