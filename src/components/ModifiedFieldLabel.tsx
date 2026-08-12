import type { ReactNode } from 'react';

export function ModifiedFieldLabel({ children, modified }: { children: ReactNode; modified: boolean }) {
  return (
    <span className="theme-text-muted font-semibold" title={modified ? 'Modified from default' : undefined}>
      {children}
    </span>
  );
}
