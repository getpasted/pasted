import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';

interface MacWindowTransparencySettingProps {
  settings: Pick<AppSettings, 'windowTransparency' | 'windowBlur'>;
  onChange: (settings: Partial<AppSettings>) => void;
}

export function MacWindowTransparencySetting({ settings, onChange }: MacWindowTransparencySettingProps) {
  const { formatNumber } = useLocalization();
  if (document.documentElement.dataset.platform !== 'macos') return null;
  const transparency = settings.windowTransparency ?? 40;
  const blur = settings.windowBlur ?? 4;

  return <div className="space-y-3 pb-1">
    <div className="flex items-center justify-between gap-4">
      <div className="min-w-0 flex-1 pe-4">
        <span className="theme-text-main block font-semibold">{translate('component.settingsGeneralPanel.windowTransparency')}</span>
        <p id="window-transparency-description" className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsGeneralPanel.windowTransparencyDescription')}</p>
      </div>
      <div className="flex shrink-0 items-center gap-3">
        <input
          id="window-transparency"
          type="range"
          min={0}
          max={100}
          step={5}
          value={transparency}
          onChange={(event) => onChange({ windowTransparency: Number(event.target.value) })}
          aria-label={translate('component.settingsGeneralPanel.windowTransparency')}
          aria-describedby="window-transparency-description"
          className="theme-range w-36"
          dir="ltr"
        />
        <output className="theme-text-muted w-10 text-end font-mono text-[11px]" htmlFor="window-transparency">
          {formatNumber(transparency / 100, { style: 'percent' })}
        </output>
      </div>
    </div>
    <div className="flex items-center justify-between gap-4">
      <div className="min-w-0 flex-1 pe-4">
        <span className="theme-text-main block font-semibold">{translate('component.settingsGeneralPanel.windowBlur')}</span>
        <p id="window-blur-description" className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsGeneralPanel.windowBlurDescription')}</p>
      </div>
      <div className="flex shrink-0 items-center gap-3">
        <input
          id="window-blur"
          type="range"
          min={0}
          max={30}
          step={2}
          value={blur}
          onChange={(event) => onChange({ windowBlur: Number(event.target.value) })}
          aria-label={translate('component.settingsGeneralPanel.windowBlur')}
          aria-describedby="window-blur-description"
          className="theme-range w-36"
          dir="ltr"
        />
        <output className="theme-text-muted w-10 text-end font-mono text-[11px]" htmlFor="window-blur">
          {translate('component.settingsGeneralPanel.valuePixels', { value: formatNumber(blur) })}
        </output>
      </div>
    </div>
  </div>;
}
