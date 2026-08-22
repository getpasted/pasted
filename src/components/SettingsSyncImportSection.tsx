import { CheckCircle2, FileWarning, LoaderCircle, RotateCcw, Upload, X } from 'lucide-react';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import type { ImportFileInspection } from './settingsSyncModel';

interface SettingsSyncImportSectionProps {
  inspection: ImportFileInspection | null;
  inspectionError: string | null;
  isInspecting: boolean;
  isImporting: boolean;
  isRestoring: boolean;
  formatDateTime: (value: string) => string;
  formatNumber: (value: number) => string;
  onChooseFile: () => void;
  onRemoveFile: () => void;
  onMerge: () => void;
  onRecover: () => void;
}

export function SettingsSyncImportSection({
  inspection,
  inspectionError,
  isInspecting,
  isImporting,
  isRestoring,
  formatDateTime,
  formatNumber,
  onChooseFile,
  onRemoveFile,
  onMerge,
  onRecover,
}: SettingsSyncImportSectionProps) {
  return <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="import-title">
    <SettingsSubsectionHeader
      id="import-title"
      title={translate('component.settingsSyncPanel.import')}
      description={translate('component.settingsSyncPanel.chooseAFileToInspectBeforeAnythingChanges')}
      actions={<ActionButton onClick={onChooseFile} disabled={isInspecting || isImporting || isRestoring} className="disabled:opacity-50">
        <Upload className="h-4 w-4" />
        {inspection ? translate('component.settingsSyncPanel.chooseAnother') : translate('component.settingsSyncPanel.chooseFile')}
      </ActionButton>}
    />
    <div className="theme-surface overflow-hidden rounded-xl border">
      {isInspecting ? (
        <div className="flex min-h-24 items-center gap-3 px-4 py-5" role="status">
          <LoaderCircle className="theme-text-muted h-5 w-5 shrink-0 animate-spin" />
          <div>
            <h4 className="theme-text-main text-[11px] font-semibold">{translate('component.settingsSyncPanel.checkingFile')}</h4>
            <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">{translate('component.settingsSyncPanel.identifyingTheDataAndValidatingItsContents')}</p>
          </div>
        </div>
      ) : inspectionError ? (
        <div className="flex min-h-24 items-center gap-3 px-4 py-4">
          <FileWarning className="theme-status-danger-text h-5 w-5 shrink-0" />
          <div className="min-w-0">
            <h4 className="theme-text-main text-[11px] font-semibold">{translate('component.settingsSyncPanel.thisFileCannotBeUsed')}</h4>
            <p className="theme-text-muted mt-1 break-words text-[10px] leading-relaxed">{inspectionError}</p>
          </div>
        </div>
      ) : inspection ? (
        <>
          <div className="flex items-start justify-between gap-4 px-4 py-3">
            <div className="flex min-w-0 items-start gap-3">
              <CheckCircle2 className="theme-status-success-text mt-0.5 h-5 w-5 shrink-0" />
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-1.5">
                  <h4 className="theme-text-main max-w-full truncate text-[11px] font-semibold" title={inspection.name}>{inspection.name}</h4>
                  <span className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide">{translate('component.settingsSyncPanel.valid')}</span>
                  <span className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide">{inspection.format === 'backup' ? translate('component.settingsSyncPanel.backup') : inspection.format}</span>
                </div>
                <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">
                  {inspection.kind === 'clips' && translate('component.settingsSyncPanel.clips')}
                  {inspection.kind === 'activity' && translate('destination.activity')}
                  {inspection.kind === 'organization' && translate('component.settingsSyncPanel.historyAndOrganization')}
                  {inspection.kind === 'backup' && translate('component.settingsSyncPanel.completeRecoveryBackup')}
                  {' · '}{inspection.sizeBytes / 1024 < 1024
                    ? translate('component.settingsSyncPanel.valueKb', { value: formatNumber(Math.max(1, Math.round(inspection.sizeBytes / 1024))) })
                    : translate('component.settingsSyncPanel.valueMb', { value: (inspection.sizeBytes / 1024 / 1024).toFixed(1) })}
                </p>
              </div>
            </div>
            <button type="button" className="theme-icon-button theme-focusable shrink-0 rounded-lg border p-1.5" onClick={onRemoveFile} aria-label={translate('component.settingsSyncPanel.removeSelectedFile')} title={translate('component.settingsSyncPanel.removeSelectedFile')}>
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
          <div className="border-t theme-divider px-4 py-3">
            <p className="theme-label text-[9px] font-semibold uppercase tracking-wider">{translate('component.settingsSyncPanel.contents')}</p>
            <p className="theme-text-main mt-1 text-[10px] leading-relaxed">
              {inspection.report && (inspection.kind === 'activity'
                ? translate('component.settingsSyncPanel.activityInspectionSummary', { scannedCount: inspection.report.scannedCount, importedCount: inspection.report.importedCount, duplicateCount: inspection.report.duplicateCount })
                : translate('component.settingsSyncPanel.clipInspectionSummary', { scannedCount: inspection.report.scannedCount, importedCount: inspection.report.importedCount, duplicateCount: inspection.report.duplicateCount }))}
              {inspection.library && translate('component.settingsSyncPanel.valueClipsValue2BinsValue3TransformsValue4Operations', { value: formatNumber(inspection.library.clipCount), value2: formatNumber(inspection.library.binCount), value3: formatNumber(inspection.library.transformCount), value4: formatNumber(inspection.library.operationCount) })}
              {inspection.backup && translate('component.settingsSyncPanel.createdValueFormatVersionFormatversion', { value: formatDateTime(inspection.backup.createdAt), formatVersion: inspection.backup.formatVersion })}
            </p>
          </div>
          <div className="theme-subtle-surface flex items-start justify-between gap-4 border-t px-4 py-3">
            <div className="min-w-0">
              <h4 className="theme-text-main text-[11px] font-semibold">{inspection.kind === 'backup' ? translate('component.settingsSyncPanel.recovery') : translate('component.settingsSyncPanel.merge')}</h4>
              <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">
                {inspection.kind === 'clips' && translate('component.settingsSyncPanel.addsNewClipsSkipsExistingMatchesAndKeepsUnrelatedData')}
                {inspection.kind === 'activity' && translate('component.settingsSyncPanel.addsInertActivityHistorySkipsDuplicatesAndNeverReplaysRecordedActions')}
                {inspection.kind === 'organization' && translate('component.settingsSyncPanel.updatesRecognizableMatchesAddsNewDataAndKeepsUnrelatedData')}
                {inspection.kind === 'backup' && translate('component.settingsSyncPanel.replacesTheCurrentStateAfterCreatingACompleteRecoveryBackup')}
              </p>
            </div>
            <ActionButton variant={inspection.kind === 'backup' ? 'danger' : 'primary'} onClick={inspection.kind === 'backup' ? onRecover : onMerge} disabled={isImporting || isRestoring} className="shrink-0 disabled:opacity-50">
              {inspection.kind === 'backup' ? <RotateCcw className="h-4 w-4" /> : <Upload className="h-4 w-4" />}
              <span>{inspection.kind === 'backup' ? translate('component.settingsSyncPanel.recover2') : translate('component.settingsSyncPanel.merge')}</span>
            </ActionButton>
          </div>
        </>
      ) : (
        <div className="flex min-h-24 items-center gap-3 px-4 py-5">
          <Upload className="theme-text-muted h-5 w-5 shrink-0" />
          <div>
            <h4 className="theme-text-main text-[11px] font-semibold">{translate('component.settingsSyncPanel.noFileSelected')}</h4>
            <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">{translate('component.settingsSyncPanel.supportedImportFiles', { backupExtension: '.pastedbackup' })}</p>
          </div>
        </div>
      )}
    </div>
  </section>;
}
