import { useState } from 'react';
import { Lock, Plus, Trash2 } from 'lucide-react';
import type { BlacklistApp } from '../types';
import { AddBlacklistAppModal } from './AddBlacklistAppModal';
import { SettingsPanelHeader } from './SettingsPanelHeader';

interface SettingsBlacklistPanelProps {
  apps: BlacklistApp[];
  onAddApp: (appName: string) => void;
  onRemoveApp: (appId: string) => void;
  onToggleRule: (appId: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreShortcuts') => void;
}

const suggestedApps = [
  {
    label: 'Security & Password Managers',
    apps: ['1Password', 'Bitwarden', 'Dashlane', 'KeePassXC', 'Enpass', 'LastPass'],
  },
  {
    label: 'Messaging & Private Chat',
    apps: ['Signal', 'Telegram', 'Slack', 'Discord', 'WhatsApp'],
  },
  {
    label: 'Web Browsers (Private Windows)',
    apps: ['Safari', 'Google Chrome', 'Firefox', 'Brave Browser', 'Arc', 'Orion'],
  },
  {
    label: 'System & Developer Tools',
    apps: ['Terminal', 'Warp', 'VS Code', 'Xcode', 'Notes', 'Mail'],
  },
];

export function SettingsBlacklistPanel({
  apps,
  onAddApp,
  onRemoveApp,
  onToggleRule,
}: SettingsBlacklistPanelProps) {
  const [isAddAppOpen, setIsAddAppOpen] = useState(false);

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Lock}
        title="App exclusions"
        description="Choose which apps Pasted ignores."
        actions={(
          <button
            type="button"
            onClick={() => setIsAddAppOpen(true)}
            className="theme-primary-button border rounded-xl px-3 py-2 text-xs font-semibold flex items-center gap-1.5 shrink-0"
          >
            <Plus className="w-4 h-4" />
            <span>Add app</span>
          </button>
        )}
      />

      <div className="space-y-2">
        {apps.length === 0 && (
          <p className="theme-text-muted theme-divider rounded-xl border border-dashed px-4 py-5 text-center text-[11px]">
            No custom app exclusions yet.
          </p>
        )}

        {apps.map((app) => (
          <div
            key={app.id}
            className="theme-surface flex items-center justify-between gap-4 p-3 rounded-xl border"
          >
            <div className="flex min-w-0 items-center space-x-3">
              <div className="settings-accent-tile w-7 h-7 shrink-0 rounded-lg border flex items-center justify-center">
                <Lock className="w-4 h-4" />
              </div>
              <span className="truncate font-semibold theme-text-main">{app.name}</span>
            </div>

            <div className="flex shrink-0 items-center space-x-4">
              <label className="flex items-center space-x-1.5 cursor-pointer theme-text-muted">
                <input
                  type="checkbox"
                  checked={app.ignoreShortcuts}
                  onChange={() => onToggleRule(app.id, 'ignoreShortcuts')}
                  className="theme-checkbox w-3.5 h-3.5 cursor-pointer rounded"
                />
                <span>Shortcuts</span>
              </label>

              <label className="flex items-center space-x-1.5 cursor-pointer theme-text-main font-medium">
                <input
                  type="checkbox"
                  checked={app.ignoreText}
                  onChange={() => onToggleRule(app.id, 'ignoreText')}
                  className="theme-checkbox w-3.5 h-3.5 cursor-pointer rounded"
                />
                <span>Text</span>
              </label>

              <label className="flex items-center space-x-1.5 cursor-pointer theme-text-main font-medium">
                <input
                  type="checkbox"
                  checked={app.ignoreImages}
                  onChange={() => onToggleRule(app.id, 'ignoreImages')}
                  className="theme-checkbox w-3.5 h-3.5 cursor-pointer rounded"
                />
                <span>Images</span>
              </label>

              <button
                type="button"
                onClick={() => onRemoveApp(app.id)}
                className="theme-danger-text theme-icon-button p-1 rounded transition-colors"
                aria-label={`Remove ${app.name} from blacklist`}
                title="Remove from Blacklist"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        ))}
      </div>

      <p className="theme-surface rounded-xl border p-4 text-[11px] theme-text-muted leading-relaxed">
        Apps that mark sensitive data as transient (like 1Password) are already ignored. Checked items will be ignored by Pasted when copying or activating Pasted global shortcuts in these apps.
      </p>

      {isAddAppOpen && (
        <AddBlacklistAppModal
          suggestions={suggestedApps}
          onAdd={onAddApp}
          onClose={() => setIsAddAppOpen(false)}
        />
      )}
    </div>
  );
}
