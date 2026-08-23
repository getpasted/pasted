import React, { useState } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  BookOpen,
  Terminal,
  Keyboard,
  Shield,
  Trash2,
  Workflow,
  ChevronRight,
  Bell,
  AudioLines,
  Radar,
  ScanText,
  History,
  type LucideIcon,
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { useToast } from './ToastProvider';
import { translate } from '../localization/runtime';
import type { HelpTopic } from '../utils/appUiState';
export type { HelpTopic } from '../utils/appUiState';

import { HelpCliTopic } from './HelpCliTopic';
import { HelpHudShortcutsTopic } from './HelpHudShortcutsTopic';

interface HelpTopicDefinition {
  id: HelpTopic;
  label: string;
  icon: LucideIcon;
  iconClassName: string;
}

const HELP_TOPICS: HelpTopicDefinition[] = [
  { id: 'getting-started', get label() { return translate('component.helpView.gettingStarted'); }, icon: BookOpen, iconClassName: 'theme-status-info-text' },
  { id: 'shortcuts-hud', get label() { return translate('component.helpView.hotkeysAndHud'); }, icon: Keyboard, iconClassName: 'theme-status-success-text' },
  { id: 'privacy-capture', get label() { return translate('component.helpView.privacyAndCapture'); }, icon: Shield, iconClassName: 'theme-status-warning-text' },
  { id: 'deletion-recovery', get label() { return translate('component.helpView.deletionAndRecovery'); }, icon: Trash2, iconClassName: 'theme-status-danger-text' },
  { id: 'analysis', get label() { return translate('component.helpView.contentAnalysis'); }, icon: Radar, iconClassName: 'theme-status-info-text' },
  { id: 'transformations', get label() { return translate('destination.transformations'); }, icon: Workflow, iconClassName: 'theme-status-info-text' },
  { id: 'cli', get label() { return translate('component.helpView.cliCommands'); }, icon: Terminal, iconClassName: 'theme-status-info-text' },
];

interface HelpViewProps {
  activeTopic: HelpTopic;
  onActiveTopicChange: (topic: HelpTopic) => void;
}

export const HelpView: React.FC<HelpViewProps> = ({ activeTopic, onActiveTopicChange }) => {
  const { showToast } = useToast();
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);

  const handleCopyCode = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCmd(code);
    setTimeout(() => setCopiedCmd(null), 1500);
  };

  const handleInstallCli = async () => {
    try {
      const res = await invoke<string>('install_cli_to_path');
      showToast({ tone: 'success', message: res });
    } catch (e: any) {
      showToast({ tone: 'error', message: String(e), durationMs: 8000 });
    }
  };

  return (
    <div className="tools-page help-page flex-1 font-sans h-screen flex flex-col overflow-hidden select-none">
      <ToolPageHeader
        icon={<BookOpen className="w-4 h-4" />}
        title={translate('destination.help')}
      />

      {/* Subpage Navigation & Content Container */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sub-Tab Sidebar Navigation */}
        <div className="help-topic-nav theme-subtle-surface">
          {HELP_TOPICS.map(({ id, label, icon: Icon, iconClassName }) => {
            const isSelected = activeTopic === id;

            return (
              <button
                key={id}
                type="button"
                onClick={() => onActiveTopicChange(id)}
                className={`help-topic-button ${isSelected ? 'is-selected' : ''}`}
                aria-current={isSelected ? 'page' : undefined}
              >
                <span className="help-topic-button__label">
                  <Icon className={iconClassName} />
                  <span>{label}</span>
                </span>
                <ChevronRight className="help-topic-button__chevron rtl:-scale-x-100" aria-hidden="true" />
              </button>
            );
          })}
        </div>

        {/* Right Detail Subpage Content */}
        <div className="tools-scroll-region flex-1 p-6 overflow-y-auto space-y-6">
          {activeTopic === 'getting-started' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title flex items-center space-x-2 text-lg font-bold">
                  <BookOpen className="h-5 w-5 theme-status-info-text" />
                  <span>{translate('component.helpView.gettingStarted')}</span>
                </h3>
                <p className="theme-text-muted mt-1 text-xs">
                  {translate('component.helpView.localHistoryIncludesCopiedTextImagesScreenshotsPdfsAndFilesCapturedWhile')}
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-title text-xs font-bold">{translate('component.helpView.theMainWindow')}</h4>
                  <ol className="theme-text-main list-inside list-decimal space-y-2 text-xs leading-relaxed">
                    <li>{translate('component.helpView.chooseHistoryACollectionOrABinFromTheLeftSidebar')}</li>
                    <li>{translate('component.helpView.selectAClipFromTheMiddleColumn')}</li>
                    <li>{translate('component.helpView.previewCopyOrganizeOrTransformItInTheRightColumn')}</li>
                  </ol>
                  <p className="theme-text-muted text-xs">{translate('component.helpView.dragTheColumnDividersToResizeTheLayoutWindowAndColumnSizes')}</p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-title text-xs font-bold">{translate('component.helpView.firstUsefulActions')}</h4>
                  <ul className="theme-text-main list-inside list-disc space-y-2 text-xs leading-relaxed">
                    <li>{translate('component.helpView.copyNormallyInAnotherAppToAddAnItemToHistory')}</li>
                    <li>{translate('component.helpView.useSearchToFindClipContentAndCollectionMetadata')}</li>
                    <li>{translate('component.helpView.rightClickAClipForQueuePinProtectNoteBinTransformAnd')}</li>
                    <li>{translate('component.helpView.openSettingsFunctionalityToChooseTheSimpleOrFullExperience')}</li>
                  </ul>
                </section>
              </div>

              <div className="theme-status-warning rounded-xl border p-4">
                <h4 className="text-xs font-bold">{translate('component.helpView.featuresNormallyHideWithoutDeletingData')}</h4>
                <p className="mt-1 text-xs leading-relaxed">{translate('component.helpView.disablingAFeatureUsuallyHidesItsInterfaceAndStopsNewBehaviorWhile')}</p>
              </div>
            </div>
          )}

          {activeTopic === 'cli' && (
            <HelpCliTopic
              copiedCmd={copiedCmd}
              onCopyCode={handleCopyCode}
              onInstallCli={handleInstallCli}
            />
          )}
          {activeTopic === 'shortcuts-hud' && <HelpHudShortcutsTopic />}

          {activeTopic === 'privacy-capture' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Shield className="w-5 h-5 theme-status-warning-text" />
                  <span>{translate('component.helpView.privacyAndCapture')}</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  {translate('component.helpView.controlWhichApplicationsAreRecordedAndHowCapturesAreConfirmedWithoutSending')}
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-info-text flex items-center space-x-2 text-xs font-bold">
                    <Keyboard className="w-4 h-4" />
                    <span>{translate('component.helpView.clipHotkeys')}</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    {translate('component.helpView.clipHotkeyDescription')}
                  </p>
                </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-warning-text text-xs font-bold">{translate('component.helpView.autoPauseAndAppExclusions')}</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    {translate('component.helpView.defaultPasswordManagerExclusions', { onePassword: translate('component.helpView.value1password'), keychain: translate('component.helpView.keychainAccess'), passwords: translate('component.helpView.passwords'), bitwarden: translate('component.helpView.bitwarden') })}</p>
                  <p className="theme-text-muted text-xs leading-relaxed">{translate('component.helpView.blockingEveryContentKindPresentsAsAnAutomaticCapturePausePartialRules')}</p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-info-text flex items-center gap-2 text-xs font-bold">
                    <Bell className="h-4 w-4" />
                    <span>{translate('component.helpView.captureFeedback')}</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">{translate('component.helpView.settingsNotificationsControlsQuietCaptureConfirmationsSkippedCaptureMessagesOptionalClipPreviews')}</p>
                  <p className="theme-text-muted text-xs leading-relaxed">{translate('component.helpView.feedbackIsRenderedLocallyAndDoesNotSendCopiedTextImagesFile')}</p>
                </section>
              </div>
            </div>
          )}

          {activeTopic === 'deletion-recovery' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Trash2 className="w-5 h-5 theme-status-danger-text" />
                  <span>{translate('component.helpView.deletionAndRecovery')}</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  {translate('component.helpView.understandWhichActionsAreRecoverableBeforeRemovingOrChangingImportantClips')}
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-danger-text text-xs font-bold">{translate('component.helpView.trashAndPermanentDeletion')}</h4>
                  <ul className="theme-text-main list-inside list-disc space-y-2 text-xs">
                    <li>{translate('component.helpView.normalDeletionDescription')}</li>
                    <li>{translate('component.helpView.restoreDescription')}</li>
                    <li>{translate('component.helpView.restoreTrashedClipsDescription')}</li>
                    <li>{translate('component.helpView.permanentDeletionDescription')}</li>
                    <li>{translate('component.helpView.protectionDescription')}</li>
                  </ul>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-info-text flex items-center gap-2 text-xs font-bold">
                    <History className="h-4 w-4" />
                    <span>{translate('component.helpView.revisionsAndFullBackups')}</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">{translate('component.helpView.revisionHistorySavesRestorableSnapshotsBeforeContentChangingEditsAndTransformReplacements')}</p>
                  <p className="theme-text-muted text-xs leading-relaxed">{translate('component.helpView.useSettingsStorageToCreateAFullBackupBeforeMajorChangesOr')}</p>
                </section>
              </div>
            </div>
          )}

          {activeTopic === 'analysis' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title flex items-center space-x-2 text-lg font-bold">
                  <Radar className="h-5 w-5 theme-status-info-text" />
                  <span>{translate('component.helpView.contentAnalysis')}</span>
                </h3>
                <p className="theme-text-muted mt-1 text-xs">{translate('component.helpView.captureAssignsAStructuralClipTypeAndRecordsSourceAttributionInspectorsMeasure')}</p>
                <p className="theme-text-muted mt-2 max-w-3xl text-xs leading-relaxed">{translate('component.helpView.analysisRunsInFourBoundedPassesInspectExtractClassifyAndSuggestEach')}</p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-info-text text-xs font-bold">{translate('component.helpView.contentClassification')}</h4>
                  <p className="theme-text-main text-xs leading-relaxed">{translate('component.helpView.enabledClassifiersRunLocallyInPriorityOrderTheLowestNumberRunsFirst')}</p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    {translate('component.helpView.classifierRescanDescription', { action: translate('component.helpView.rescanClips') })}</p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-info-text text-xs font-bold">{translate('component.helpView.structuralInspection')}</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    {translate('component.helpView.structureRecordsContentFreeFactsSuchAsTextCountsImageDimensionsFile')}
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    {translate('component.helpView.fileAvailabilityAndTotalSizeAreLiveObservationsTheyAreCheckedWhen')}
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">{translate('component.helpView.anInstalledFfprobeOrMediainfoExecutableAlsoSuppliesBoundedContainerCodecStream')}</p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <Terminal className="h-4 w-4" />
                    <span>{translate('component.helpView.customExtractors')}</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    {translate('component.helpView.customExtractorRecipeDescription')}</p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    {translate('component.helpView.newCustomCommandsBeginDisabledReviewTheSelectedExecutableBeforeEnablingAutomatic')}
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <AudioLines className="h-4 w-4" />
                    <span>{translate('component.helpView.audioTranscription')}</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    {translate('component.helpView.whisperTranscriptionUsesAnInstalledWhisperCppExecutableAndASelectedLocal')}
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    {translate('component.helpView.storedTranscriptsAreSearchableAndDoNotReplaceFileReferencesModelsAre')}
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <ScanText className="h-4 w-4" />
                    <span>{translate('component.helpView.opticalCharacterRecognition')}</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">{translate('component.helpView.ocrUsesAppleVisionOnMacosOrAnInstalledTesseract5Executable')}</p>
                  <p className="theme-text-muted text-xs leading-relaxed">{translate('component.helpView.ocrStatusDescription', { command: 'pasted ocr status --json' })}
                  </p>
                </section>
              </div>
            </div>
          )}

          {activeTopic === 'transformations' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Workflow className="w-5 h-5 theme-status-info-text" />
                  <span>{translate('destination.transformations')}</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  {translate('component.helpView.describeTheResultOnceSaveItAsATransformThenReuseIt')}
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="theme-status-info-text text-xs font-bold">{translate('component.helpView.availableTransformations')}</h4>
                <div className="theme-text-main grid grid-cols-2 gap-2 text-xs font-mono">
                  <div className="theme-code-surface p-2 rounded border">{translate('component.helpView.uppercaseLowercase')}</div>
                  <div className="theme-code-surface p-2 rounded border">{translate('component.helpView.titleCaseCamelcase')}</div>
                  <div className="theme-code-surface p-2 rounded border">{translate('component.helpView.trimWhitespace')}</div>
                  <div className="theme-code-surface p-2 rounded border">{translate('component.helpView.smartPunctuation')}</div>
                  <div className="theme-code-surface p-2 rounded border">{translate('component.helpView.urlEncodeDecode')}</div>
                  <div className="theme-code-surface p-2 rounded border">{translate('component.helpView.jsonPrettify')}</div>
                </div>
              </div>

              <div className="theme-subtle-surface rounded-xl border p-4">
                <h4 className="text-xs font-bold">{translate('component.helpView.advancedTransformationTools')}</h4>
                <p className="theme-text-muted mt-1 text-xs">{translate('component.helpView.operationsAreDeterministicBuildingBlocksForReusableTransformsManuallyBuiltTransformsRetain')}</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
