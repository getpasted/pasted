import { Sliders, Command, Shield, Cloud } from 'lucide-react';
import { startWindowDrag } from '../utils/windowDrag';

export type SettingsTab = 'general' | 'hotkeys' | 'blacklist' | 'sync';

interface SettingsTabsProps {
  activeTab: SettingsTab;
  onChange: (tab: SettingsTab) => void;
}

const TABS = [
  { id: 'general', label: 'General', Icon: Sliders },
  { id: 'hotkeys', label: 'Hotkeys', Icon: Command },
  { id: 'blacklist', label: 'Blacklist', Icon: Shield },
  { id: 'sync', label: 'Sync', Icon: Cloud },
] as const;

export function SettingsTabs({ activeTab, onChange }: SettingsTabsProps) {
  return (
    <div onMouseDown={startWindowDrag} className="flex items-center justify-center">
      <div className="theme-panel flex items-center p-1 rounded-xl border space-x-1 titlebar-no-drag">
        {TABS.map(({ id, label, Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            aria-pressed={activeTab === id}
            className={`flex flex-col items-center justify-center px-4 py-2 rounded-lg text-xs font-semibold transition-all border ${
              activeTab === id
                ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                : 'settings-tab-idle border-transparent text-gray-400'
            }`}
          >
            <Icon className="w-4 h-4 mb-1" />
            <span>{label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
