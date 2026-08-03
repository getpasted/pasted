import { useState, type FormEvent } from 'react';
import { Lock, Plus, Trash2 } from 'lucide-react';
import type { BlacklistApp } from '../types';

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
  const [appName, setAppName] = useState('');

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    const name = appName.trim();
    if (!name) return;
    onAddApp(name);
    setAppName('');
  };

  return (
    <div className="settings-panel theme-panel p-6 rounded-2xl border space-y-4 text-xs">
      <h4 className="font-bold text-gray-100 uppercase tracking-wider text-[11px]">
        Ignore from the Following Apps:
      </h4>

      <div className="space-y-2 max-h-72 overflow-y-auto pr-1">
        {apps.length === 0 && (
          <p className="theme-text-muted rounded-xl border border-dashed border-gray-700/80 px-4 py-5 text-center text-[11px]">
            No custom app exclusions yet.
          </p>
        )}

        {apps.map((app) => (
          <div
            key={app.id}
            className="theme-surface flex items-center justify-between gap-4 p-3 rounded-xl border"
          >
            <div className="flex min-w-0 items-center space-x-3">
              <div className="w-7 h-7 shrink-0 rounded-lg bg-gray-800 border border-gray-700 flex items-center justify-center">
                <Lock className="w-4 h-4 text-amber-400" />
              </div>
              <span className="truncate font-semibold text-gray-200">{app.name}</span>
            </div>

            <div className="flex shrink-0 items-center space-x-4">
              <label className="flex items-center space-x-1.5 cursor-pointer text-gray-400">
                <input
                  type="checkbox"
                  checked={app.ignoreShortcuts}
                  onChange={() => onToggleRule(app.id, 'ignoreShortcuts')}
                  className="w-3.5 h-3.5 accent-[#007aff] cursor-pointer rounded"
                />
                <span>Shortcuts</span>
              </label>

              <label className="flex items-center space-x-1.5 cursor-pointer text-gray-200 font-medium">
                <input
                  type="checkbox"
                  checked={app.ignoreText}
                  onChange={() => onToggleRule(app.id, 'ignoreText')}
                  className="w-3.5 h-3.5 accent-[#007aff] cursor-pointer rounded"
                />
                <span>Text</span>
              </label>

              <label className="flex items-center space-x-1.5 cursor-pointer text-gray-200 font-medium">
                <input
                  type="checkbox"
                  checked={app.ignoreImages}
                  onChange={() => onToggleRule(app.id, 'ignoreImages')}
                  className="w-3.5 h-3.5 accent-[#007aff] cursor-pointer rounded"
                />
                <span>Images</span>
              </label>

              <button
                type="button"
                onClick={() => onRemoveApp(app.id)}
                className="p-1 text-gray-500 hover:text-red-400 rounded hover:bg-gray-800 transition-colors"
                aria-label={`Remove ${app.name} from blacklist`}
                title="Remove App"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        ))}
      </div>

      <form onSubmit={handleSubmit} className="space-y-2 pt-2 border-t border-gray-700/80">
        <select
          aria-label="Suggested app"
          onChange={(event) => setAppName(event.target.value)}
          value={suggestedApps.some((group) => group.apps.includes(appName)) ? appName : ''}
          className="theme-input w-full border rounded-lg px-3 py-1.5 text-xs focus:outline-none focus:border-gray-500 truncate"
        >
          <option value="" disabled>-- Select Installed or Popular App --</option>
          {suggestedApps.map((group) => (
            <optgroup key={group.label} label={group.label}>
              {group.apps.map((name) => <option key={name} value={name}>{name}</option>)}
            </optgroup>
          ))}
        </select>

        <div className="flex items-center space-x-2">
          <input
            type="text"
            aria-label="Custom app name"
            placeholder="Or type custom app name (e.g. Signal, Bitwarden)..."
            value={appName}
            onChange={(event) => setAppName(event.target.value)}
            className="theme-input flex-1 border rounded-lg px-3 py-1.5 text-xs focus:outline-none focus:border-gray-500"
          />
          <button
            type="submit"
            disabled={!appName.trim()}
            className="flex items-center space-x-1 px-3.5 py-1.5 bg-white hover:bg-gray-200 text-black font-semibold rounded-lg transition-[background-color,opacity,transform] text-xs shadow-md active:scale-95 disabled:cursor-not-allowed disabled:opacity-50"
            title="Add App to Blacklist"
          >
            <Plus className="w-4 h-4" />
            <span>Add App</span>
          </button>
        </div>
      </form>

      <p className="text-[11px] text-gray-400 leading-relaxed pt-2">
        Apps that mark sensitive data as transient (like 1Password) are already ignored. Checked items will be ignored by Pasted when copying or activating Pasted global shortcuts in these apps.
      </p>
    </div>
  );
}
