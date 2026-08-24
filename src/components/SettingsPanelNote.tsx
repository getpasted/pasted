import type { ReactNode } from 'react';

interface SettingsPanelNoteProps {
  action?: ReactNode;
  children: ReactNode;
}

export function SettingsPanelNote({ action, children }: SettingsPanelNoteProps) {
  return (
    <div className="theme-surface theme-text-muted flex items-center justify-between gap-4 rounded-xl border p-4 text-[11px] leading-relaxed">
      <p className="min-w-0 flex-1">{children}</p>
      {action}
    </div>
  );
}
