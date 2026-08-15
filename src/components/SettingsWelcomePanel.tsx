import { PlayCircle } from 'lucide-react';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';

interface SettingsWelcomePanelProps {
  onOpen: () => void;
}

export function SettingsWelcomePanel({ onOpen }: SettingsWelcomePanelProps) {
  return (
    <section className="theme-panel rounded-2xl border p-4">
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-start gap-3">
          <SettingsAccentTile>
            <PlayCircle className="h-4 w-4" />
          </SettingsAccentTile>
          <div className="min-w-0 pt-0.5">
            <h3 className="theme-title text-sm font-bold">Welcome, Copycat</h3>
            <p className="theme-text-muted mt-1 text-[11px] leading-relaxed">
              Revisit the Copycat Covenant, migration, shared workspace, and keyboard access.
            </p>
          </div>
        </div>
        <ActionButton onClick={onOpen} className="shrink-0 cursor-pointer">
          Open Copycat Welcome…
        </ActionButton>
      </div>
    </section>
  );
}
