import type { ReactNode } from 'react';

interface SettingsPanelNoteProps {
  children: ReactNode;
}

export function SettingsPanelNote({ children }: SettingsPanelNoteProps) {
  return (
    <p className="theme-surface theme-text-muted rounded-xl border p-4 text-[11px] leading-relaxed">
      {children}
    </p>
  );
}
