import type { ReactNode } from 'react';
import { SettingsAccentTile } from './SettingsAccentTile';

interface SettingsSubsectionHeaderProps {
  title: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  actions?: ReactNode;
  id?: string;
  className?: string;
}

export function SettingsSubsectionHeader({
  title,
  description,
  icon,
  actions,
  id,
  className = '',
}: SettingsSubsectionHeaderProps) {
  return (
    <div className={`flex items-start gap-3 ${className}`.trim()}>
      {icon && (
        <SettingsAccentTile>{icon}</SettingsAccentTile>
      )}
      <div className="min-w-0 flex-1">
        <h3 id={id} className="theme-title text-sm font-bold">{title}</h3>
        {description && <p className="theme-text-muted mt-1 text-[11px] leading-relaxed">{description}</p>}
      </div>
      {actions}
    </div>
  );
}
