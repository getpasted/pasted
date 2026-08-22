import { FolderInput, ShieldAlert, ShieldCheck, ShieldQuestion } from 'lucide-react';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import type { LibraryLocationInfo, StorageProtectionInfo } from './settingsSyncModel';

interface SettingsSyncLibrarySectionProps {
  location: LibraryLocationInfo | null;
  storageProtection: StorageProtectionInfo | null;
  isMoving: boolean;
  onMove: () => void;
  onRestoreDefault: () => void;
}

export function SettingsSyncLibrarySection({
  location,
  storageProtection,
  isMoving,
  onMove,
  onRestoreDefault,
}: SettingsSyncLibrarySectionProps) {
  return <section className="space-y-3" aria-labelledby="library-location-title">
    <SettingsSubsectionHeader
      id="library-location-title"
      title={translate('component.settingsSyncPanel.databaseLocation')}
      description={translate('component.settingsSyncPanel.chooseWhereEverythingIsStored')}
      actions={<div className="flex shrink-0 items-center gap-2">
        {location && !location.isDefault && (
          <ActionButton onClick={onRestoreDefault} disabled={isMoving} className="disabled:opacity-50">
            {translate('component.settingsSyncPanel.useDefault')}
          </ActionButton>
        )}
        <ActionButton onClick={onMove} disabled={isMoving} className="disabled:opacity-50">
          <FolderInput className="h-4 w-4" />
          <span>{isMoving ? translate('component.settingsSyncPanel.moving') : translate('component.settingsSyncPanel.move')}</span>
        </ActionButton>
      </div>}
    />
    <div className="theme-surface overflow-hidden rounded-xl border">
      <div className="p-3">
        <p className="theme-label text-[10px] font-bold uppercase tracking-wider">
          {location?.isDefault ? translate('component.settingsSyncPanel.defaultLocation') : translate('component.settingsSyncPanel.customLocation')}
        </p>
        <p className="theme-text-main mt-1 select-text truncate font-mono text-[11px]" title={location?.path}>
          {location?.path ?? translate('component.settingsSyncPanel.loadingDatabaseLocation')}
        </p>
      </div>
      <div className="theme-subtle-surface flex min-h-[4.5rem] items-start gap-3 border-t theme-divider px-3 py-3">
        {storageProtection?.status === 'protected'
          ? <ShieldCheck className="theme-status-success-text mt-0.5 h-4 w-4 shrink-0" />
          : storageProtection?.status === 'notDetected'
            ? <ShieldAlert className="theme-status-warning-text mt-0.5 h-4 w-4 shrink-0" />
            : <ShieldQuestion className="theme-text-muted mt-0.5 h-4 w-4 shrink-0" />}
        <div className="min-w-0">
          <p className="theme-label text-[9px] font-bold uppercase tracking-wider">{translate('component.settingsSyncPanel.storageProtection')}</p>
          <p className="theme-text-main mt-0.5 text-[11px] font-semibold">
            {storageProtection?.summary ?? translate('component.settingsSyncPanel.checkingVolumeEncryption')}
          </p>
          <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">
            {storageProtection?.detail ?? translate('component.settingsSyncPanel.checkingTheActiveDatabaseVolume')}
          </p>
        </div>
      </div>
    </div>
  </section>;
}
