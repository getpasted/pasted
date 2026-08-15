import type { ReactNode } from 'react';

interface RegistryPanelHeaderProps {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
}

export function RegistryPanelHeader({
  title,
  description,
  actions,
}: RegistryPanelHeaderProps) {
  return (
    <div className="theme-divider flex min-h-[49px] shrink-0 items-center justify-between gap-3 border-b p-2">
      <div className="min-w-0 px-1">
        <h3 className="theme-text-main text-xs font-semibold">{title}</h3>
        {description && (
          <p className="theme-text-muted mt-0.5 text-[10px]">{description}</p>
        )}
      </div>
      {actions}
    </div>
  );
}
