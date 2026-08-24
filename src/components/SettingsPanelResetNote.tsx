import { RotateCcw } from 'lucide-react';
import type { ReactNode } from 'react';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { SettingsPanelNote } from './SettingsPanelNote';

export function SettingsPanelResetNote({
  children,
  disabled = false,
  onReset,
}: {
  children: ReactNode;
  disabled?: boolean;
  onReset: () => void;
}) {
  return (
    <SettingsPanelNote
      action={(
        <ActionButton onClick={onReset} disabled={disabled} className="shrink-0">
          <RotateCcw className="h-3.5 w-3.5" aria-hidden="true" />
          {translate('common.resetWithEllipsis')}
        </ActionButton>
      )}
    >
      {children}
    </SettingsPanelNote>
  );
}
