import type { HTMLAttributes } from 'react';

function joinClasses(...classes: Array<string | undefined | false>) {
  return classes.filter(Boolean).join(' ');
}

export function RegistryPanelFooter({
  className,
  align = 'between',
  ...props
}: HTMLAttributes<HTMLDivElement> & { align?: 'end' | 'between' }) {
  return (
    <div
      className={joinClasses(
        'theme-divider flex min-h-12 items-center gap-2 border-t p-2',
        align === 'end' ? 'justify-end' : 'justify-between',
        className,
      )}
      {...props}
    />
  );
}
