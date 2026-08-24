import { RotateCcw, Trash2 } from 'lucide-react';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { ActionButton } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
interface SelectOption {
  value: string;
  label: string;
}

interface SettingsGeneralRetentionSectionsProps {
  settings: AppSettings;
  trashAgeOptions: SelectOption[];
  trashCountOptions: SelectOption[];
  activityAgeOptions: SelectOption[];
  activityCountOptions: SelectOption[];
  trashedClipCount: number;
  isRestoringTrash: boolean;
  isAltPressed: boolean;
  canRestoreTrash: boolean;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
  onRestoreTrash: () => void;
  onClearHistory?: (permanent: boolean) => void;
}

export function SettingsGeneralRetentionSections({
  settings,
  trashAgeOptions,
  trashCountOptions,
  activityAgeOptions,
  activityCountOptions,
  trashedClipCount,
  isRestoringTrash,
  isAltPressed,
  canRestoreTrash,
  onUpdateSettings,
  onRestoreTrash,
  onClearHistory,
}: SettingsGeneralRetentionSectionsProps) {
  return <>
    {settings.enableTrash && <>
      <div className="theme-divider border-t" />
      <div className="space-y-4">
        <SettingsSubsectionHeader
          title={translate('component.settingsGeneralPanel.trash')}
          description={translate('component.settingsGeneralPanel.controlHowMuchDeletedHistoryRemainsRecoverable')}
        />
        <div className="theme-surface overflow-hidden rounded-xl border">
          <div className="flex items-center justify-between gap-4 px-3 py-2.5">
            <div className="min-w-0">
              <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.keepTrashedClipsFor')}</span>
              <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.olderTrashedClipsArePermanentlyPurged')}</p>
            </div>
            <MenuSelect value={String(settings.trashAgeDays)} options={trashAgeOptions} onChange={(value) => onUpdateSettings({ trashAgeDays: Number(value) })} label={translate('component.settingsGeneralPanel.maximumTrashAge')} className="settings-menu-select w-40 shrink-0" />
          </div>
          <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
            <div className="min-w-0">
              <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumTrashedClips')}</span>
              <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.theOldestEligibleItemsArePermanentlyPurgedFirst')}</p>
            </div>
            <MenuSelect value={String(settings.trashCapacityCount)} options={trashCountOptions} onChange={(value) => onUpdateSettings({ trashCapacityCount: Number(value) })} label={translate('component.settingsGeneralPanel.maximumTrashedClipsRetained')} className="settings-menu-select w-40 shrink-0" />
          </div>
          <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">{translate('component.settingsGeneralPanel.bothLimitsApplyProtectedClipsAreAlwaysKept')}</p>
        </div>
        <div className="theme-surface overflow-hidden rounded-xl border">
          <div className="flex items-start justify-between gap-4 px-3 py-2.5">
            <div className="min-w-0">
              <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.restoreTrashedClips')}</span>
              <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.returnEveryTrashedClipToHistory')}</p>
            </div>
            <ActionButton onClick={onRestoreTrash} disabled={!canRestoreTrash || trashedClipCount === 0 || isRestoringTrash} className="shrink-0 disabled:cursor-not-allowed disabled:opacity-40">
              <RotateCcw className="h-3.5 w-3.5" />
              <span>{isRestoringTrash ? translate('component.settingsGeneralPanel.restoring') : translate('component.settingsGeneralPanel.restoreTrashedClips')}</span>
            </ActionButton>
          </div>
          <div className="theme-divider flex items-start justify-between gap-4 border-t px-3 py-2.5">
            <div className="min-w-0">
              <span className={`font-semibold block ${isAltPressed ? 'theme-danger-text' : 'theme-text-main'}`}>
                {isAltPressed ? translate('component.settingsGeneralPanel.deleteAllClips') : translate('component.settingsGeneralPanel.trashAllClips')}
              </span>
              <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                {isAltPressed
                  ? translate('component.settingsGeneralPanel.permanentlyDeleteAllUnpinnedAndUnprotectedClips')
                  : translate('component.settingsGeneralPanel.moveAllUnpinnedAndUnprotectedClipsToTrashHoldOptionToPermanently')}
              </p>
            </div>
            <button type="button" onClick={(event) => onClearHistory?.(event.altKey)} className="theme-status-danger flex shrink-0 items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold border transition-colors cursor-pointer">
              <Trash2 className="w-3.5 h-3.5" />
              <span>{isAltPressed ? translate('component.settingsGeneralPanel.deleteAllClips2') : translate('component.settingsGeneralPanel.trashAllClips2')}</span>
            </button>
          </div>
        </div>
      </div>
    </>}

    {settings.enableActivityLog && <>
      <div className="theme-divider border-t" />
      <div className="space-y-4">
        <SettingsSubsectionHeader
          title={translate('component.settingsGeneralPanel.activityHistory')}
          description={translate('component.settingsGeneralPanel.chooseHowMuchActivityHistoryToKeep')}
        />
        <div className="theme-surface overflow-hidden rounded-xl border">
          <div className="flex items-center justify-between gap-4 px-3 py-2.5">
            <div className="min-w-0">
              <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.keepActivityFor')}</span>
              <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.olderActivityEntriesAreRemovedAutomatically')}</p>
            </div>
            <MenuSelect value={String(settings.activityLogAgeDays)} options={activityAgeOptions} onChange={(value) => onUpdateSettings({ activityLogAgeDays: Number(value) })} label={translate('component.settingsGeneralPanel.maximumActivityAge')} className="settings-menu-select w-40 shrink-0" />
          </div>
          <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
            <div className="min-w-0">
              <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumActivityEntries')}</span>
              <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{translate('component.settingsGeneralPanel.theOldestEntriesAreRemovedFirst')}</p>
            </div>
            <MenuSelect value={String(settings.activityLogCapacity)} options={activityCountOptions} onChange={(value) => onUpdateSettings({ activityLogCapacity: Number(value) })} label={translate('component.settingsGeneralPanel.maximumActivityEntriesRetained')} className="settings-menu-select w-40 shrink-0" />
          </div>
          <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">{translate('component.settingsGeneralPanel.bothLimitsApplyUnlimitedAndForeverDisableAutomaticRemoval')}</p>
        </div>
      </div>
    </>}
  </>;
}
