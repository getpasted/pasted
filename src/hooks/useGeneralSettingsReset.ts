import { createElement, useState } from 'react';
import type { LocaleDefinition } from '../localization/runtime';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { generalDefaultUpdates } from '../generalSettingsDefaults';
import { generalSettingsResetChanges } from '../generalSettingsResetChanges';
import { SettingsResetChanges } from '../components/SettingsResetChanges';
import type { ConfirmationDialogRequest } from '../components/ConfirmationDialog';

export function useGeneralSettingsReset({ settings, locales, onUpdateSettings, onResetColumnWidths }: {
  settings: AppSettings;
  locales: readonly LocaleDefinition[];
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
  onResetColumnWidths?: () => void;
}) {
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const requestReset = () => {
    const customColumnWidths = Boolean(
      localStorage.getItem('pasted_sidebar_width') || localStorage.getItem('pasted_list_width'),
    );
    const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent || navigator.platform);
    const changes = generalSettingsResetChanges(settings, locales, customColumnWidths, isMac);
    setConfirmation({
      title: translate('component.settingsGeneralPanel.resetGeneral'),
      description: translate('component.settingsResetChanges.description'),
      details: createElement(SettingsResetChanges, { changes }),
      confirmLabel: translate('common.reset'),
      confirmDisabled: changes.length === 0,
      onConfirm: () => {
        onUpdateSettings(generalDefaultUpdates());
        if (customColumnWidths) onResetColumnWidths?.();
        setConfirmation(null);
      },
    });
  };
  return { confirmation, requestReset, setConfirmation };
}
