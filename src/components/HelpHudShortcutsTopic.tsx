import { Command, Info, Keyboard, Trash2, Zap } from 'lucide-react';

import { translate } from '../localization/runtime';
import { hudPrimaryModifierLabel } from './quickHudModel';

export function HelpHudShortcutsTopic() {
  const modifier = hudPrimaryModifierLabel(document.documentElement.dataset.platform);
  return <div className="space-y-6 animate-in fade-in">
    <div>
      <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
        <Keyboard className="w-5 h-5 theme-status-success-text" />
        <span>{translate('component.helpView.hotkeysAndHud')}</span>
      </h3>
      <p className="theme-text-muted text-xs mt-1">
        {translate('component.helpView.useTheDefaultHotkeysBelowOrChangeAndDisableThemUnderSettings')}
      </p>
    </div>

    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div className="theme-panel p-4 rounded-xl border space-y-2">
        <div className="theme-status-warning-text flex items-center space-x-2 text-xs font-bold">
          <Trash2 className="w-4 h-4 theme-status-danger-text" />
          <span>{translate('component.helpView.optionAltKeyPermanentDelete')}</span>
        </div>
        <p className="theme-text-muted text-xs">
          {translate('component.helpView.permanentDeleteShortcutDescription', { modifier: translate('component.helpView.option'), symbol: 'X' })}
        </p>
      </div>

      <div className="theme-panel p-4 rounded-xl border space-y-2">
        <div className="theme-status-info-text flex items-center space-x-2 text-xs font-bold">
          <Command className="w-4 h-4" />
          <span>{translate('component.helpView.openHud')}</span>
        </div>
        <p className="theme-text-muted text-xs">
          {translate('component.helpView.openHudHotkeyDescription', { hotkey: '⌥ Shift V' })}
        </p>
      </div>

      <div className="theme-panel p-4 rounded-xl border space-y-2">
        <div className="theme-status-info-text flex items-center space-x-2 text-xs font-bold">
          <Zap className="w-4 h-4" />
          <span>{translate('component.helpView.hudNumberKeys19')}</span>
        </div>
        <p className="theme-text-muted text-xs">
          {translate('component.helpView.hudNumberShortcutDescription', { modifier, start: 1, end: 9 })}
        </p>
      </div>

      <div className="theme-panel p-4 rounded-xl border space-y-2">
        <div className="theme-status-success-text flex items-center space-x-2 text-xs font-bold">
          <Info className="w-4 h-4" />
          <span>{translate('component.helpView.escapeKeyDismiss')}</span>
        </div>
        <p className="theme-text-muted text-xs">
          {translate('component.helpView.dismissHudShortcutDescription', { key: translate('component.helpView.esc') })}
        </p>
      </div>
    </div>
  </div>;
}
