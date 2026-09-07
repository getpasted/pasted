import { useEffect, useId, useState } from 'react';
import { Minus, Plus } from 'lucide-react';
import { useLocalization } from '../localization/LocalizationProvider';

interface SettingsNumberStepperProps {
  label: string;
  description?: string;
  value: number;
  min: number;
  max: number;
  disabled: boolean;
  onChange: (value: number) => void;
}

export function SettingsNumberStepper({ label, description, value, min, max, disabled, onChange }: SettingsNumberStepperProps) {
  const id = useId();
  const { t } = useLocalization();
  const [draft, setDraft] = useState(String(value));
  useEffect(() => { setDraft(String(value)); }, [value]);
  const normalize = () => {
    const parsed = /^\d+$/.test(draft) ? Number(draft) : value;
    const normalized = Math.min(max, Math.max(min, parsed));
    setDraft(String(normalized));
    if (normalized !== value) onChange(normalized);
  };
  const adjust = (amount: number) => {
    const current = /^\d+$/.test(draft) ? Number(draft) : value;
    const next = Math.min(max, Math.max(min, current + amount));
    setDraft(String(next));
    if (next !== value) onChange(next);
  };
  return <div className="flex items-start justify-between gap-4 text-xs">
    <div className="min-w-0 flex-1">
      <span id={`${id}-label`} className="font-semibold theme-text-main block">{label}</span>
      {description && <p id={`${id}-description`} className="text-[11px] theme-text-muted leading-normal mt-0.5">{description}</p>}
    </div>
    <div className="theme-surface flex shrink-0 items-center overflow-hidden rounded-lg border" role="group" aria-labelledby={`${id}-label`}>
      <button type="button" disabled={disabled || value <= min}
        className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-e disabled:cursor-not-allowed disabled:opacity-35"
        aria-label={t('numberStepper.decrease', { label })}
        title={t('numberStepper.decrease', { label })}
        onMouseDown={event => event.preventDefault()} onClick={() => adjust(-1)}>
        <Minus className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
      </button>
      <input id={id} type="number" inputMode="numeric" min={min} max={max} step="1" required value={draft} disabled={disabled}
        aria-labelledby={`${id}-label`}
        aria-describedby={description ? `${id}-description` : undefined}
        className="settings-number-input theme-secondary-button h-8 w-14 rounded-none border-0 px-2 text-center font-mono text-[10px] font-semibold tabular-nums"
        onChange={event => { if (/^\d*$/.test(event.target.value)) setDraft(event.target.value); }}
        onBlur={normalize}
        onKeyDown={event => {
          if (event.key === 'Enter') event.currentTarget.blur();
          if (event.key === 'Escape') { setDraft(String(value)); event.currentTarget.blur(); }
          if (!event.ctrlKey && !event.metaKey && ['e', 'E', '+', '-', '.', ','].includes(event.key)) event.preventDefault();
        }} />
      <button type="button" disabled={disabled || value >= max}
        className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-s disabled:cursor-not-allowed disabled:opacity-35"
        aria-label={t('numberStepper.increase', { label })}
        title={t('numberStepper.increase', { label })}
        onMouseDown={event => event.preventDefault()} onClick={() => adjust(1)}>
        <Plus className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
      </button>
    </div>
  </div>;
}
