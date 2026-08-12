import type { ReactNode } from 'react';

const sizeClasses = {
  small: 'h-7 w-7 rounded-lg',
  compact: 'h-8 w-8 rounded-lg',
  medium: 'h-9 w-9 rounded-lg',
  large: 'h-10 w-10 rounded-xl',
} as const;

export function SettingsAccentTile({
  children,
  size = 'medium',
  className = '',
}: {
  children: ReactNode;
  size?: keyof typeof sizeClasses;
  className?: string;
}) {
  return (
    <span className={`settings-accent-tile flex shrink-0 items-center justify-center border ${sizeClasses[size]} ${className}`.trim()}>
      {children}
    </span>
  );
}
