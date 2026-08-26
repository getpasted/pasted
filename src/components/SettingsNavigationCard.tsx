import { ChevronRight } from 'lucide-react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { SettingsAccentTile } from './SettingsAccentTile';

export function SettingsNavigationCard({
  icon,
  title,
  description,
  className = '',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  icon: ReactNode;
  title: ReactNode;
  description: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`theme-card-idle theme-interactive-card flex w-full items-center gap-3 border px-3 py-3 text-start ${className}`.trim()}
      {...props}
    >
      <SettingsAccentTile>{icon}</SettingsAccentTile>
      <span className="min-w-0 flex-1">
        <span className="theme-title block text-sm font-bold">{title}</span>
        <span className="theme-text-muted mt-0.5 block text-xs">{description}</span>
      </span>
      <ChevronRight className="theme-text-muted h-4 w-4 shrink-0 rtl:-scale-x-100" aria-hidden="true" />
    </button>
  );
}
