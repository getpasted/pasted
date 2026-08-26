import type { ButtonHTMLAttributes } from 'react';
import { translate } from '../localization/runtime';

interface SettingsSwitchProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'aria-label'> {
  checked: boolean;
  label: string;
  busy?: boolean;
  tone?: 'default' | 'danger';
  ariaLabel?: string;
}

export function SettingsSwitch({
  checked,
  label,
  busy = false,
  tone = 'default',
  ariaLabel,
  className = '',
  disabled,
  ...props
}: SettingsSwitchProps) {
  const isDisabled = disabled || busy;

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel ?? (checked
        ? translate('component.settingsSwitch.disableLabel', { label })
        : translate('component.settingsSwitch.enableLabel', { label }))}
      disabled={isDisabled}
      className={`settings-switch relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent ${tone === 'danger' ? 'is-danger' : ''} ${checked ? 'is-on' : ''} ${busy ? 'disabled:cursor-wait disabled:opacity-50' : 'disabled:cursor-not-allowed'} ${className}`.trim()}
      {...props}
    >
      <span className={`settings-switch-thumb elevation-control pointer-events-none inline-block h-4 w-4 rounded-full transition-transform ${checked ? 'ltr:translate-x-4 rtl:-translate-x-4' : 'translate-x-0'}`} />
    </button>
  );
}
