import { Minus, Plus, RotateCcw } from 'lucide-react';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { ACTUAL_SIZE, APP_ZOOM_STEPS, appZoomPercent, stepAppZoom } from '../utils/appZoom';
import { ActionButton } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
const rowHeightOptions = [
  { value: 'small', get label() { return translate('component.settingsGeneralPanel.compact'); } },
  { value: 'medium', get label() { return translate('component.settingsGeneralPanel.standard'); } },
  { value: 'large', get label() { return translate('component.settingsGeneralPanel.spacious'); } },
];
const startupViewOptions = [
  { value: 'last_active', get label() { return translate('component.settingsGeneralPanel.lastActivePage'); } },
  { value: 'clip_history', get label() { return translate('component.settingsGeneralPanel.clipHistory'); } },
];

interface SettingsGeneralLayoutSectionProps {
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
  onResetColumnWidths?: () => void;
}

export function SettingsGeneralLayoutSection({ settings, onUpdateSettings, onResetColumnWidths }: SettingsGeneralLayoutSectionProps) {
  const resetColumnWidths = () => {
    if (onResetColumnWidths) {
      onResetColumnWidths();
      return;
    }
    localStorage.removeItem('pasted_sidebar_width');
    localStorage.removeItem('pasted_list_width');
    window.location.reload();
  };

  return <div className="space-y-4">
    <SettingsSubsectionHeader
      title={translate('component.settingsGeneralPanel.layout')}
      description={translate('component.settingsGeneralPanel.adjustAppScalingClipDensityAndWorkspaceDimensions')}
    />
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0 flex-1">
        <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.zoom')}</span>
        <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.adjustTheSizeOfNavigationControlsAndClipContent')}</p>
      </div>
      <div className="theme-surface flex shrink-0 items-center overflow-hidden rounded-lg border" role="group" aria-label={translate('component.settingsGeneralPanel.applicationZoom')}>
        <button type="button" aria-label={translate('component.settingsGeneralPanel.zoomOut')} title={translate('component.settingsGeneralPanel.zoomOut2')} disabled={settings.textSize <= APP_ZOOM_STEPS[0]} onClick={() => onUpdateSettings({ textSize: stepAppZoom(settings.textSize, -1) })} className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-e disabled:cursor-not-allowed disabled:opacity-35">
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button type="button" aria-label={translate('component.settingsGeneralPanel.actualSize')} title={translate('component.settingsGeneralPanel.actualSize0')} onClick={() => onUpdateSettings({ textSize: ACTUAL_SIZE })} className="theme-secondary-button h-8 min-w-14 border-0 px-2 font-mono text-[10px] font-semibold">
          {appZoomPercent(settings.textSize)}%
        </button>
        <button type="button" aria-label={translate('component.settingsGeneralPanel.zoomIn')} title={translate('component.settingsGeneralPanel.zoomIn2')} disabled={settings.textSize >= APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1]} onClick={() => onUpdateSettings({ textSize: stepAppZoom(settings.textSize, 1) })} className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-s disabled:cursor-not-allowed disabled:opacity-35">
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
    <div className="flex items-start justify-between">
      <div className="pe-4 flex-1 min-w-0">
        <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.clipDensity')}</span>
        <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.adjustsClipSpacingTextDepthAndPreviewSizeThroughoutTheHistoryList')}</p>
      </div>
      <MenuSelect value={settings.rowHeight} options={rowHeightOptions} onChange={(value) => onUpdateSettings({ rowHeight: value as AppSettings['rowHeight'] })} label={translate('component.settingsGeneralPanel.clipDensity2')} className="settings-menu-select" />
    </div>
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0 flex-1">
        <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.startupView')}</span>
        <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.reopenTheLastViewOrAlwaysStartInClipHistory')}</p>
      </div>
      <MenuSelect value={settings.startupView} options={startupViewOptions} onChange={(value) => onUpdateSettings({ startupView: value as AppSettings['startupView'] })} label={translate('component.settingsGeneralPanel.startupView2')} className="settings-menu-select" />
    </div>
    <div className="flex items-start justify-between">
      <div className="pe-4 flex-1 min-w-0">
        <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.columnWidths')}</span>
        <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.resetsTheLeftSidebarAndMiddleHistoryListPanelWidthsToTheir')}</p>
      </div>
      <ActionButton onClick={resetColumnWidths} className="shrink-0 cursor-pointer">
        <RotateCcw className="w-3.5 h-3.5" />
        <span>{translate('component.settingsGeneralPanel.resetColumnWidths')}</span>
      </ActionButton>
    </div>
  </div>;
}
