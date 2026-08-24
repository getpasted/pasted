import { ArrowRight, Plus, Trash2 } from 'lucide-react';
import { translate } from '../localization/runtime';

export interface SettingsResetChange {
  label: string;
  before: string | null;
  after: string | null;
}

export function resetBooleanLabel(value: boolean) {
  return translate(value ? 'component.settingsResetChanges.on' : 'component.settingsResetChanges.off');
}

export function SettingsResetChanges({ changes }: { changes: SettingsResetChange[] }) {
  if (changes.length === 0) {
    return <p className="theme-text-muted text-xs">{translate('component.settingsResetChanges.alreadyAtDefaults')}</p>;
  }

  return (
    <div className="space-y-2">
      <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] gap-2 px-2 text-[9px] font-bold uppercase tracking-wider theme-text-subtle">
        <span>{translate('component.settingsResetChanges.current')}</span>
        <span aria-hidden="true" />
        <span>{translate('component.settingsResetChanges.afterReset')}</span>
      </div>
      <ul className="theme-surface theme-divide max-h-64 divide-y overflow-y-auto rounded-xl border">
        {changes.map(({ label, before, after }, index) => (
          <li key={`${label}-${before}-${after}-${index}`} className="space-y-1.5 px-2.5 py-2">
            <div className="truncate text-[10px] font-semibold theme-text-main">{label}</div>
            <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2 text-[11px]">
              <span className={before === null ? 'theme-text-subtle italic' : 'theme-status-danger-text line-through'}>
                {before ?? translate('component.settingsResetChanges.notPresent')}
              </span>
              {before === null
                ? <span className="theme-status-success-text"><Plus className="h-3.5 w-3.5" aria-hidden="true" /><span className="sr-only">{translate('component.settingsResetChanges.added')}</span></span>
                : after === null
                  ? <span className="theme-status-danger-text"><Trash2 className="h-3.5 w-3.5" aria-hidden="true" /><span className="sr-only">{translate('component.settingsResetChanges.removed')}</span></span>
                  : <ArrowRight className="h-3.5 w-3.5 theme-text-subtle rtl:rotate-180" aria-hidden="true" />}
              <span className={after === null ? 'theme-text-subtle italic' : 'theme-status-success-text font-semibold'}>
                {after ?? translate('component.settingsResetChanges.notPresent')}
              </span>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
