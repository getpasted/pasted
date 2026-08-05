import type { ReactNode } from 'react';
import type { LucideIcon } from 'lucide-react';

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
    <div className="settings-section-header flex items-start justify-between gap-4">
      <div className="flex min-w-0 items-start gap-3">
        <span className="settings-accent-tile flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border">
          <Icon className="h-5 w-5" />
        </span>
        <div className="min-w-0 pt-0.5">
          <h2 className="theme-title text-sm font-bold">{title}</h2>
          <p className="theme-text-muted mt-1 text-xs leading-relaxed">{description}</p>
        </div>
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}
