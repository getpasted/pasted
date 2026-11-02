import type { AppSettings } from '../types';
import { translate } from '../localization/runtime';
import { detectDesktopPlatform } from '../utils/platform';
import { MenuSelect } from './MenuSelect';

interface SettingsGeneralTrayControlsProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
}

const iconStyleOptions = [
  { value: 'clipboard', get label() { return translate('component.settingsGeneralPanel.clipboard'); } },
  { value: 'copycat', get label() { return translate('component.settingsGeneralPanel.copycat'); } },
];

export function SettingsGeneralTrayControls({ settings, onUpdateSettings }: SettingsGeneralTrayControlsProps) {
  const platform = detectDesktopPlatform();
  const isMac = platform === 'macos';
  const dockIconOptions = isMac
    ? [
        { value: 'auto_hide', get label() { return translate('component.settingsGeneralPanel.autoHideDockIcon'); } },
        { value: 'both', get label() { return translate('component.settingsGeneralPanel.alwaysShowDockAndMenuBar'); } },
        { value: 'menubar_only', get label() { return translate('component.settingsGeneralPanel.menubarIconOnly'); } },
      ]
    : [
        { value: 'auto_hide', get label() { return translate('component.settingsGeneralPanel.autoHideTaskbarIcon'); } },
        { value: 'both', get label() { return translate('component.settingsGeneralPanel.alwaysShowTrayAndTaskbar'); } },
        { value: 'menubar_only', get label() { return translate('component.settingsGeneralPanel.systemTrayIconOnly'); } },
      ];
  const placementLabel = isMac
    ? translate('component.settingsGeneralPanel.dockAndMenuBarIcon')
    : translate('component.settingsGeneralPanel.systemTrayAndTaskbar');
  const iconStyleLabel = isMac
    ? translate('component.settingsGeneralPanel.menuBarIcon')
    : translate('component.settingsGeneralPanel.systemTrayIcon');

  return (
    <>
      <div className="flex items-center justify-between pt-1">
        <span className="font-medium">{placementLabel}</span>
        <MenuSelect
          value={settings.dockMenubarIcon}
          options={dockIconOptions}
          onChange={(value) => onUpdateSettings({ dockMenubarIcon: value as AppSettings['dockMenubarIcon'] })}
          label={isMac ? translate('component.settingsGeneralPanel.dockAndMenuBarIconBehavior') : placementLabel}
          className="settings-menu-select"
        />
      </div>

      {(isMac || platform === 'linux') && (
        <div className="flex items-start justify-between gap-4 pt-1">
          <div className="min-w-0 flex-1 pe-4">
            <span className="font-semibold theme-text-main block">{iconStyleLabel}</span>
            <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
              {translate('component.settingsGeneralPanel.chooseTheClassicClipboardOrTheResidentCopycat')}
            </p>
          </div>
          <MenuSelect
            value={settings.menubarIconStyle}
            options={iconStyleOptions}
            onChange={(value) => onUpdateSettings({ menubarIconStyle: value as AppSettings['menubarIconStyle'] })}
            label={isMac ? translate('component.settingsGeneralPanel.menuBarIcon2') : iconStyleLabel}
            className="settings-menu-select shrink-0"
          />
        </div>
      )}
    </>
  );
}
