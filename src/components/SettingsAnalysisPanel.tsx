import { Clipboard, Lightbulb, ScanSearch } from 'lucide-react';
import { useState } from 'react';
import { useAnalysisMaintenance } from '../hooks/useAnalysisMaintenance';
import { translate } from '../localization/runtime';
import { AnalysisLifecycleSequence } from './AnalysisLifecycleSequence';
import { ActionButton } from './AppDialogLayout';
import { BuiltinLifecycleManagerDialog } from './BuiltinLifecycleManagerDialog';
import { ClassifierManagerDialog } from './ClassifierManagerDialog';
import { ConfirmationDialog } from './ConfirmationDialog';
import { ContentExtractorManagerDialog } from './ContentExtractorManagerDialog';
import { useContentTypes } from './ContentTypeProvider';
import { SearchIndexManagerDialog } from './SearchIndexManagerDialog';
import { SettingsOcrPanel } from './SettingsOcrPanel';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsPanelResetNote } from './SettingsPanelResetNote';
import type { SettingsAnalysisPanelProps } from './settingsAnalysisTypes';
export function SettingsAnalysisPanel({
  contentClassificationEnabled,
  fileFormatsEnabled,
  ocrEnabled,
  transcriptionsEnabled,
  transformationsEnabled,
  typesEnabled,
  sourcesEnabled,
  searchEnabled,
  onOpenIntelligence,
  onSearchClips,
}: SettingsAnalysisPanelProps) {
  const { refresh: refreshContentTypes, refreshGroups } = useContentTypes();
  const [isCaptureManagerOpen, setIsCaptureManagerOpen] = useState(false);
  const [isInspectorManagerOpen, setIsInspectorManagerOpen] = useState(false);
  const [isExtractorManagerOpen, setIsExtractorManagerOpen] = useState(false);
  const [isIndexManagerOpen, setIsIndexManagerOpen] = useState(false);
  const [isClassifierManagerOpen, setIsClassifierManagerOpen] = useState(false);
  const [isSuggestionManagerOpen, setIsSuggestionManagerOpen] = useState(false);
  const {
    confirmation,
    rescanning,
    rescanHistory,
    restoring,
    restoreAnalysis,
    setConfirmation,
  } = useAnalysisMaintenance({
    contentClassificationEnabled,
    fileFormatsEnabled,
    refreshContentTypes,
    refreshGroups,
  });

  return <div className="space-y-5 text-xs">
    <SettingsPanelHeader
      icon={ScanSearch}
      title={translate('component.settingsAnalysisPanel.analysis')}
      description={translate('component.settingsAnalysisPanel.automaticallyScanClipsAndIndexTheirContents')}
      actions={(contentClassificationEnabled || fileFormatsEnabled) ? <ActionButton onClick={rescanHistory} disabled={rescanning}>
        <ScanSearch className="h-3.5 w-3.5" /> {rescanning ? translate('component.settingsAnalysisPanel.rescanning') : translate('component.settingsAnalysisPanel.rescanClips')}
      </ActionButton> : undefined}
    />
    <AnalysisLifecycleSequence
      contentClassificationEnabled={contentClassificationEnabled}
      searchEnabled={searchEnabled}
      transformationsEnabled={transformationsEnabled}
      typesEnabled={typesEnabled}
      onOpenCapture={() => setIsCaptureManagerOpen(true)}
      onOpenInspector={() => setIsInspectorManagerOpen(true)}
      onOpenExtractor={() => setIsExtractorManagerOpen(true)}
      onOpenIndex={() => setIsIndexManagerOpen(true)}
      onOpenClassifier={() => setIsClassifierManagerOpen(true)}
      onOpenSuggestion={() => setIsSuggestionManagerOpen(true)}
    />
    <SettingsPanelResetNote onReset={restoreAnalysis} disabled={restoring}>
      {translate('component.settingsAnalysisPanel.notAllStepsRunForAllClipsSomeStepsMayBeLong')}
    </SettingsPanelResetNote>
    <ClassifierManagerDialog
      isOpen={isClassifierManagerOpen}
      onClose={() => setIsClassifierManagerOpen(false)}
    />
    <BuiltinLifecycleManagerDialog
      isOpen={isCaptureManagerOpen}
      onClose={() => setIsCaptureManagerOpen(false)}
      kind="capture"
      title={translate('component.settingsAnalysisPanel.capture')}
      description={translate('component.settingsAnalysisPanel.reviewClipTypeAndContextRecordedBeforeAnalysisBegins')}
      icon={Clipboard}
      sourcesEnabled={sourcesEnabled}
    />
    <BuiltinLifecycleManagerDialog
      isOpen={isInspectorManagerOpen}
      onClose={() => setIsInspectorManagerOpen(false)}
      kind="inspector"
      title={translate('component.settingsAnalysisPanel.inspectors')}
      description={translate('component.settingsAnalysisPanel.reviewClipInspectionBehaviorAndMediaAvailability')}
      icon={ScanSearch}
      fileFormatsEnabled={fileFormatsEnabled}
    />
    <ContentExtractorManagerDialog
      isOpen={isExtractorManagerOpen}
      onClose={() => setIsExtractorManagerOpen(false)}
      ocrEnabled={ocrEnabled}
      transcriptionsEnabled={transcriptionsEnabled}
      onOpenIntelligence={onOpenIntelligence ? () => {
        setIsExtractorManagerOpen(false);
        onOpenIntelligence();
      } : undefined}
    />
    <SearchIndexManagerDialog
      isOpen={isIndexManagerOpen}
      onClose={() => setIsIndexManagerOpen(false)}
    />
    <BuiltinLifecycleManagerDialog
      isOpen={isSuggestionManagerOpen}
      onClose={() => setIsSuggestionManagerOpen(false)}
      kind="suggestion"
      title={translate('component.settingsAnalysisPanel.suggestions')}
      description={translate('component.settingsAnalysisPanel.reviewSmartActionSuggestions')}
      icon={Lightbulb}
    />
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
    {ocrEnabled && <SettingsOcrPanel extractorRevision={Number(isExtractorManagerOpen)} searchEnabled={searchEnabled} onSearchClips={onSearchClips} />}
  </div>;
}
