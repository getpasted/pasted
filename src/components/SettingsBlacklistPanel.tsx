import { useRef, useState } from 'react';
import { Check, ChevronDown, Lock, Plus, Trash2 } from 'lucide-react';
import type { BlacklistApp } from '../types';
import { AddBlacklistAppModal } from './AddBlacklistAppModal';
import { AnchoredMenu, MenuDivider, MenuItem } from './AnchoredMenu';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { OverflowText } from './OverflowText';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';
import { SettingsPanelNote } from './SettingsPanelNote';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { ConnectedMenuAction } from './ConnectedMenuAction';

interface SettingsBlacklistPanelProps {
  apps: BlacklistApp[];
  onAddApp: (appName: string) => void;
  onRemoveApp: (appId: string) => void;
  onToggleRule: (appId: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreFiles' | 'ignoreHotkeys') => void;
}

const suggestedApps = [
  {
    label: 'Security and password managers',
    apps: ['1Password', 'Bitwarden', 'Dashlane', 'KeePassXC', 'Enpass', 'LastPass'],
  },
  {
    label: 'Messaging and private chat',
    apps: ['Signal', 'Telegram', 'Slack', 'Discord', 'WhatsApp'],
  },
  {
    label: 'Web Browsers (Private Windows)',
    apps: ['Safari', 'Google Chrome', 'Firefox', 'Brave Browser', 'Arc', 'Orion'],
  },
  {
    label: 'System and developer tools',
    apps: ['Terminal', 'Warp', 'VS Code', 'Xcode', 'Notes', 'Mail'],
  },
];

type ExclusionRule = 'ignoreText' | 'ignoreImages' | 'ignoreFiles' | 'ignoreHotkeys';

const exclusionOptions: Array<{ label: string; rule: ExclusionRule }> = [
  { label: 'Text', rule: 'ignoreText' },
  { label: 'Images', rule: 'ignoreImages' },
  { label: 'Files', rule: 'ignoreFiles' },
];

function AppExclusionMenu({
  app,
  onRemove,
  onToggle,
}: {
  app: BlacklistApp;
  onRemove: () => void;
  onToggle: (rule: ExclusionRule) => void;
}) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const captureLabels = exclusionOptions.filter(({ rule }) => app[rule]).map(({ label }) => label);
  const activeCount = captureLabels.length + Number(app.ignoreHotkeys);
  const summary = activeCount === 0
    ? 'Nothing'
    : captureLabels.length === exclusionOptions.length && app.ignoreHotkeys
      ? 'Everything'
      : captureLabels.length === exclusionOptions.length
        ? 'All content'
        : [...captureLabels, ...(app.ignoreHotkeys ? ['Hotkeys'] : [])].join(', ');

  const renderOption = ({ label, rule }: { label: string; rule: ExclusionRule }) => {
    const active = app[rule];
    return (
      <MenuItem
        key={rule}
        role="menuitemcheckbox"
        aria-checked={active}
        active={active}
        className="gap-2 px-2.5 py-2"
        onClick={() => onToggle(rule)}
      >
        <span className="min-w-0 flex-1">{label}</span>
        <span className="grid h-3.5 w-3.5 shrink-0 place-items-center" aria-hidden="true">
          {active && <Check className="h-3.5 w-3.5" />}
        </span>
      </MenuItem>
    );
  };

  return (
    <>
      <ConnectedMenuAction
        className="w-48"
        groupLabel={`Exclusions for ${app.name}`}
        actionLabel={`Remove ${app.name} from App Exclusions`}
        action={<Trash2 className="h-3.5 w-3.5" aria-hidden="true" />}
        danger
        onAction={onRemove}
      >
        <button
          ref={triggerRef}
          type="button"
          className="menu-select-trigger theme-focusable flex min-w-0 flex-1 items-center gap-2 rounded-l-lg border px-2.5 text-left"
          aria-label={`Choose exclusions for ${app.name}`}
          aria-haspopup="menu"
          aria-expanded={isOpen}
          onClick={() => setIsOpen((open) => !open)}
        >
          <span className="min-w-0 flex-1 truncate py-2 text-xs font-semibold">{summary}</span>
          <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
        </button>
      </ConnectedMenuAction>

      {isOpen && (
        <AnchoredMenu
          anchor={{ kind: 'element', ref: triggerRef, align: 'end' }}
          ariaLabel={`Exclusions for ${app.name}`}
          onClose={() => setIsOpen(false)}
          style={{ width: 208 }}
        >
          <div className="theme-text-subtle px-2.5 pb-1 pt-1 text-[10px] font-bold uppercase tracking-wider">
            Content
          </div>
          {exclusionOptions.map(renderOption)}
          <MenuDivider />
          <div className="theme-text-subtle px-2.5 pb-1 pt-1 text-[10px] font-bold uppercase tracking-wider">
            Actions
          </div>
          {renderOption({ label: 'Hotkeys', rule: 'ignoreHotkeys' })}
        </AnchoredMenu>
      )}
    </>
  );
}

export function SettingsBlacklistPanel({
  apps,
  onAddApp,
  onRemoveApp,
  onToggleRule,
}: SettingsBlacklistPanelProps) {
  const [isAddAppOpen, setIsAddAppOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const requestRemove = (app: BlacklistApp) => {
    setConfirmation({
      title: 'Remove app exclusion?',
      description: `${app.name} will no longer be excluded.`,
      details: 'Enabled capture and hotkeys will resume while this app is focused.',
      confirmLabel: 'Remove',
      tone: 'danger',
      onConfirm: () => {
        onRemoveApp(app.id);
        setConfirmation(null);
      },
    });
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Lock}
        title="App Exclusions"
        description="Choose which apps to ignore."
        actions={(
          <ActionButton
            variant="primary"
            onClick={() => setIsAddAppOpen(true)}
            className="shrink-0"
          >
            <Plus className="w-4 h-4" />
            <span>Add app…</span>
          </ActionButton>
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
              <SettingsAccentTile size="small">
                <Lock className="w-4 h-4" />
              </SettingsAccentTile>
              <OverflowText text={app.name} className="truncate font-semibold theme-text-main" />
            </div>

            <AppExclusionMenu
              app={app}
              onToggle={(rule) => onToggleRule(app.id, rule)}
              onRemove={() => requestRemove(app)}
            />
          </div>
        ))}
      </div>

      <SettingsPanelNote>
        Common password managers, including 1Password, are excluded by default. Checked content is not captured, and checked hotkeys do not activate while these apps are focused.
      </SettingsPanelNote>

      {isAddAppOpen && (
        <AddBlacklistAppModal
          suggestions={suggestedApps}
          onAdd={onAddApp}
          onClose={() => setIsAddAppOpen(false)}
        />
      )}
      <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
    </div>
  );
}
