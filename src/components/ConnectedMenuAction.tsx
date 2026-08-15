import type { ReactNode } from 'react';

interface ConnectedMenuActionProps {
  action: ReactNode;
  actionLabel: string;
  children: ReactNode;
  groupLabel: string;
  onAction: () => void;
  className?: string;
  danger?: boolean;
}

export function ConnectedMenuAction({
  action,
  actionLabel,
  children,
  groupLabel,
  onAction,
  className = '',
  danger = false,
}: ConnectedMenuActionProps) {
  return (
    <div
      className={`connected-menu-action flex min-w-0 [&>button:last-child]:-ml-px ${className}`}
      role="group"
      aria-label={groupLabel}
    >
      {children}
      <button
        type="button"
        className={`theme-icon-button theme-focusable flex shrink-0 items-center justify-center gap-1.5 rounded-r-lg border px-2.5 text-xs font-semibold ${danger ? 'theme-danger-text' : ''}`}
        aria-label={actionLabel}
        title={actionLabel}
        onClick={onAction}
      >
        {action}
      </button>
    </div>
  );
}
