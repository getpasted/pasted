import type { ReactNode } from 'react';

export function RegistryEditorShell({ children, className = '' }: { children: ReactNode; className?: string }) {
  return (
    <div className="@container">
      <div className={`grid min-h-0 grid-cols-1 gap-4 @4xl:min-h-[520px] @4xl:grid-cols-[minmax(220px,0.72fr)_minmax(0,1.4fr)] ${className}`}>
        {children}
      </div>
    </div>
  );
}
