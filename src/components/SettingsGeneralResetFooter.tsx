import type { LocaleDefinition } from '../localization/runtime';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { useGeneralSettingsReset } from '../hooks/useGeneralSettingsReset';
import { ConfirmationDialog } from './ConfirmationDialog';
import { SettingsPanelResetNote } from './SettingsPanelResetNote';

export function SettingsGeneralResetFooter(props: {
  settings: AppSettings;
  locales: readonly LocaleDefinition[];
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
  onResetColumnWidths?: () => void;
}) {
  const { confirmation, requestReset, setConfirmation } = useGeneralSettingsReset(props);
  return <>
    <SettingsPanelResetNote onReset={requestReset}>
      {translate('component.settingsGeneralPanel.resetGeneralNote')}
    </SettingsPanelResetNote>
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  </>;
}
