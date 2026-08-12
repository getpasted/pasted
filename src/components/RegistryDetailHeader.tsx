import type { ReactNode } from 'react';

interface RegistryDetailHeaderProps {
  icon: ReactNode;
  title: ReactNode;
  meta: ReactNode;
  trailing?: ReactNode;
  iconClassName?: string;
}

export function RegistryDetailHeader({
  icon,
  title,
  meta,
  trailing,
  iconClassName = '',
}: RegistryDetailHeaderProps) {
  return (
    <div className="theme-divider flex items-start gap-3 border-b pb-4">
      <span className={`theme-badge grid h-10 w-10 shrink-0 place-items-center rounded-lg border ${iconClassName}`.trim()}>
        {icon}
      </span>
      <div className="min-w-0 flex-1">
        <h3 className="theme-text-main truncate text-xs font-bold">{title}</h3>
        <p className="theme-text-muted mt-1 text-[10px]">{meta}</p>
      </div>
      {trailing}
    </div>
  );
}
