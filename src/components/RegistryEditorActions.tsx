import type { ReactNode } from 'react';

export function RegistryEditorActions({
  leading,
  trailing,
  className = '',
}: {
  leading?: ReactNode;
  trailing?: ReactNode;
  className?: string;
}) {
  return (
    <div className={`theme-divider mt-auto flex flex-wrap items-center gap-2 border-t pt-3 ${className}`}>
      {leading && <div className="flex flex-wrap items-center gap-2">{leading}</div>}
      {trailing && <div className="ms-auto flex flex-wrap items-center justify-end gap-2">{trailing}</div>}
    </div>
  );
}
