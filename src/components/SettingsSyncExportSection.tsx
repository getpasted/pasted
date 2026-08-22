import { Download } from 'lucide-react';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { SettingsSwitch } from './SettingsSwitch';
import type { ExportDataId, ExportFormat, ExportMode, VisibleExportFormat } from './settingsSyncModel';

const EXPORT_EXTENSION: Record<VisibleExportFormat, string> = { json: '.json', csv: '.csv', backup: '.pastedbackup' };
const EXPORT_FORMAT_LABEL: Record<VisibleExportFormat, string> = { json: 'JSON', csv: 'CSV', backup: 'BACKUP' };
const EXPORT_FORMAT_DESCRIPTION: Record<VisibleExportFormat, string> = {
  json: 'Preserves rich data.',
  csv: 'Creates spreadsheet-ready records.',
  backup: 'Includes everything for recovery.',
};
const EXPORT_DATA: ReadonlyArray<{
  id: ExportDataId;
  label: string;
  description: string;
  formats: readonly VisibleExportFormat[];
  nested?: boolean;
}> = [
  { id: 'clips', get label() { return translate('component.settingsSyncPanel.clips'); }, get description() { return translate('component.settingsSyncPanel.historyRichContentNotesProtectionPinsAndCaptureDetails'); }, formats: ['json', 'csv', 'backup'] },
  { id: 'organization', get label() { return translate('component.settingsSyncPanel.organization'); }, get description() { return translate('component.settingsSyncPanel.addsTrashBinsTransformsOperationsContentTypesClassifiersAndOcr'); }, formats: ['json', 'backup'], nested: true },
  { id: 'activity', get label() { return translate('destination.activity'); }, get description() { return translate('component.settingsSyncPanel.portableAuditRecordsWithoutClipboardContentsOrActionReplay'); }, formats: ['json', 'csv', 'backup'] },
  { id: 'settings', get label() { return translate('component.settingsSyncPanel.settingsAndApplicationData'); }, get description() { return translate('component.settingsSyncPanel.settingsHotkeysAppExclusionRulesQueueStateAndConnectionConfiguration'); }, formats: ['backup'] },
  { id: 'recovery', get label() { return translate('component.settingsSyncPanel.revisionsAndAutomationHistory'); }, get description() { return translate('component.settingsSyncPanel.clipRevisionsAutomationsAndExecutionHistory'); }, formats: ['backup'] },
  { id: 'interface', get label() { return translate('component.settingsSyncPanel.interfaceAndWindowState'); }, get description() { return translate('component.settingsSyncPanel.savedLayoutNavigationAndWindowState'); }, formats: ['backup'] },
];

interface SettingsSyncExportSectionProps {
  activityEnabled: boolean;
  exportMode: ExportMode;
  exportFormat: ExportFormat;
  exportData: Record<ExportDataId, boolean>;
  isExporting: boolean;
  onChooseMode: (mode: ExportMode) => void;
  onChooseFormat: (format: ExportFormat) => void;
  onToggleData: (id: ExportDataId) => void;
  onExport: () => void;
}

export function SettingsSyncExportSection({
  activityEnabled,
  exportMode,
  exportFormat,
  exportData,
  isExporting,
  onChooseMode,
  onChooseFormat,
  onToggleData,
  onExport,
}: SettingsSyncExportSectionProps) {
  const activeFormat: VisibleExportFormat = exportMode === 'full' ? 'backup' : exportFormat;
  const fileCount = Number(exportData.clips) + Number(exportData.activity);

  return <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="export-title">
    <SettingsSubsectionHeader
      id="export-title"
      title={translate('component.settingsSyncPanel.export')}
      description={translate('component.settingsSyncPanel.chooseWhatToIncludeAndHowToPackageIt')}
      actions={<div className="theme-code-surface flex shrink-0 rounded-lg border p-1" aria-label={translate('component.settingsSyncPanel.exportFormat')}>
        {(['json', 'csv', 'backup'] as const satisfies readonly VisibleExportFormat[]).map((format) => {
          const active = exportMode === 'full' ? format === 'backup' : format === exportFormat;
          return <button
            key={format}
            type="button"
            aria-pressed={active}
            onClick={() => {
              if (format === 'backup') {
                onChooseMode('full');
                return;
              }
              onChooseFormat(format);
              onChooseMode('custom');
            }}
            className={`settings-feature-preset rounded-md px-3 py-1.5 text-[10px] font-semibold uppercase ${active ? 'is-active' : ''}`}
          >
            {EXPORT_FORMAT_LABEL[format]}
          </button>;
        })}
      </div>}
    />
    <div className="theme-surface overflow-hidden rounded-xl border">
      <div className="divide-y theme-divide">
        {EXPORT_DATA.filter((item) => item.id !== 'activity' || activityEnabled || exportMode === 'full').map((item) => {
          const supported = item.formats.includes(activeFormat);
          const checked = exportMode === 'full' || (supported && exportData[item.id]);
          const disabled = exportMode === 'full' || !supported;
          return <div key={item.id} className={`relative flex items-start justify-between gap-4 py-3 pe-4 ${item.nested ? 'ps-10' : 'ps-4'} ${disabled ? 'settings-disabled-row' : ''}`}>
            {item.nested && <span aria-hidden="true" className="theme-divider absolute start-4 top-0 h-1/2 w-3 rounded-es-md border-b border-s" />}
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h4 className="theme-text-main text-[11px] font-semibold">{item.label}</h4>
                <span className="flex items-center gap-1" aria-label={translate('component.settingsSyncPanel.supportedFormatsValue', { value: item.formats.map((format) => EXPORT_FORMAT_LABEL[format]).join(', ') })}>
                  {item.formats.map((format) => <span key={format} className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold tracking-wide">{EXPORT_FORMAT_LABEL[format]}</span>)}
                </span>
              </div>
              <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">{item.description}</p>
            </div>
            <SettingsSwitch checked={checked} label={item.label} disabled={disabled} onClick={() => onToggleData(item.id)} />
          </div>;
        })}
      </div>
      <div className="theme-subtle-surface flex items-start justify-between gap-4 border-t px-4 py-3">
        <div className="min-w-0">
          <p className="theme-text-muted text-[10px] leading-relaxed">
            {exportMode === 'full'
              ? translate('component.settingsSyncPanel.fullBackupFileSummary', { extension: '.pastedbackup' })
              : translate('component.settingsSyncPanel.exportFileSummary', { count: fileCount, extension: EXPORT_EXTENSION[exportFormat] })}
          </p>
          <dl className="mt-2 grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1 text-[9px] leading-relaxed">
            <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.scope')}</dt>
            <dd className="theme-text-muted">{EXPORT_FORMAT_DESCRIPTION[activeFormat]}</dd>
            <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.originalFiles')}</dt>
            <dd className="theme-text-muted">{translate('component.settingsSyncPanel.remainInTheirCurrentLocations')}</dd>
            <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.credentials')}</dt>
            <dd className="theme-text-muted">{translate('component.settingsSyncPanel.areNotCopied')}</dd>
            <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.encryption')}</dt>
            <dd className="theme-text-muted">{translate('component.settingsSyncPanel.none')}</dd>
          </dl>
        </div>
        <ActionButton variant="primary" onClick={onExport} disabled={isExporting || (exportMode === 'custom' && fileCount === 0)} className="shrink-0 disabled:opacity-50">
          <Download className="h-4 w-4" />
          <span>{isExporting ? translate('component.settingsSyncPanel.exporting') : translate('component.settingsSyncPanel.export2')}</span>
        </ActionButton>
      </div>
    </div>
  </section>;
}
