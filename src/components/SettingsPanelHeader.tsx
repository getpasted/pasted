import type { ReactNode } from 'react';
import type { LucideIcon } from 'lucide-react';
import { SettingsAccentTile } from './SettingsAccentTile';

interface SettingsPanelHeaderProps {
  icon: LucideIcon;
  title: string;
  description: string;
  actions?: ReactNode;
}

export function SettingsPanelHeader({
  icon: Icon,
  title,
  description,
  actions,
}: SettingsPanelHeaderProps) {
  return (
    <div className="settings-section-header flex flex-wrap items-start justify-between gap-4">
      <div className="flex min-w-[min(16rem,100%)] flex-1 items-start gap-3">
        <SettingsAccentTile size="large">
          <Icon className="h-5 w-5" />
        </SettingsAccentTile>
        <div className="min-w-0 pt-0.5">
          <h2 className="theme-title text-sm font-bold">{title}</h2>
          <p className="theme-text-muted mt-1 text-xs leading-relaxed">{description}</p>
        </div>
      </div>
      {actions && <div className="ml-auto flex max-w-full flex-wrap items-center justify-end gap-2">{actions}</div>}
    </div>
  );
}
