import { Sliders, Command, Shield, Cloud, Cable, Bug } from 'lucide-react';

export type SettingsTab = 'general' | 'hotkeys' | 'connections' | 'blacklist' | 'sync' | 'debug';

interface SettingsTabsProps {
  activeTab: SettingsTab;
  onChange: (tab: SettingsTab) => void;
}

const TABS = [
  { id: 'general', label: 'General', Icon: Sliders },
  { id: 'hotkeys', label: 'Hotkeys', Icon: Command },
  { id: 'connections', label: 'Connections', Icon: Cable },
  { id: 'blacklist', label: 'Blacklist', Icon: Shield },
  { id: 'sync', label: 'Sync', Icon: Cloud },
  { id: 'debug', label: 'Debug', Icon: Bug },
] as const;

export function SettingsTabs({ activeTab, onChange }: SettingsTabsProps) {
  return (
    <nav className="theme-surface settings-tabs flex items-center gap-1 rounded-xl border p-1" aria-label="Settings sections">
      {TABS.map(({ id, label, Icon }) => (
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
