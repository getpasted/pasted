import { Minus, Plus } from 'lucide-react';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { ACTUAL_SIZE, APP_ZOOM_STEPS, appZoomPercent, stepAppZoom } from '../utils/appZoom';

interface SettingsGeneralZoomSettingProps {
  settings: AppSettings;
  onChange: (settings: Partial<AppSettings>) => void;
}

export function SettingsGeneralZoomSetting({ settings, onChange }: SettingsGeneralZoomSettingProps) {
  return <div className="flex items-start justify-between gap-4">
    <div className="min-w-0 flex-1">
      <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.zoom')}</span>
      <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.adjustTheSizeOfNavigationControlsAndClipContent')}</p>
    </div>
    <div className="theme-surface flex shrink-0 items-center overflow-hidden rounded-lg border" role="group" aria-label={translate('component.settingsGeneralPanel.applicationZoom')}>
      <button type="button" aria-label={translate('component.settingsGeneralPanel.zoomOut')} title={translate('component.settingsGeneralPanel.zoomOut2')} disabled={settings.textSize <= APP_ZOOM_STEPS[0]} onClick={() => onChange({ textSize: stepAppZoom(settings.textSize, -1) })} className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-e disabled:cursor-not-allowed disabled:opacity-35">
        <Minus className="h-3.5 w-3.5" />
      </button>
      <button type="button" aria-label={translate('component.settingsGeneralPanel.actualSize')} title={translate('component.settingsGeneralPanel.actualSize0')} onClick={() => onChange({ textSize: ACTUAL_SIZE })} className="theme-secondary-button h-8 min-w-14 border-0 px-2 font-mono text-[10px] font-semibold">
        {appZoomPercent(settings.textSize)}%
      </button>
      <button type="button" aria-label={translate('component.settingsGeneralPanel.zoomIn')} title={translate('component.settingsGeneralPanel.zoomIn2')} disabled={settings.textSize >= APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1]} onClick={() => onChange({ textSize: stepAppZoom(settings.textSize, 1) })} className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-s disabled:cursor-not-allowed disabled:opacity-35">
        <Plus className="h-3.5 w-3.5" />
      </button>
    </div>
  </div>;
}
