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
  Copy,
  Check,
  Zap,
  Info,
  Command,
  Download,
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

const CLI_SYMLINK_COMMAND = 'sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted /usr/local/bin/pasted';
const CLI_ALIAS_COMMAND = 'alias pasted="/Applications/Pasted.app/Contents/MacOS/pasted"';

const CLI_COMMAND_GROUPS = [
  {
    get title() { return translate('component.helpView.history'); },
    commands: [
      { usage: 'pasted copy "Hello"', get description() { return translate('component.helpView.saveATextClipOmitTheArgumentToReadStdin'); } },
      { usage: 'cat server.log | pasted copy', get description() { return translate('component.helpView.pipeBoundedTextIntoClipboardHistory'); } },
      { usage: 'pasted list [--limit N] [--offset N] [--bin ID | --pinned | --trash] [--json]', get description() { return translate('component.helpView.listABoundedPageFromHistoryTrashABinOrPinnedClips'); } },
      { usage: 'pasted search [query] [--clip TYPE] [--content TYPE] [--format FORMAT] [--source APP] [--trash] [--limit N] [--offset N] [--json]', get description() { return translate('component.helpView.searchABoundedPageOfHistoryOrTrashWithCollectionFilters'); } },
      { usage: 'pasted import sources [--json]', get description() { return translate('component.helpView.listSupportedExternalHistorySourcesAndTheirDetectedLocations'); } },
      { usage: 'pasted import <alfred|pastebot|pasta|paste|copyclip|maccy|flycut> [path] [--json]', get description() { return translate('component.helpView.mergeTextHistoryFromAnotherClipboardManagerSkippingDuplicates'); } },
      { usage: 'pasted retention [--count N] [--days N] [--trash-count N] [--trash-days N] [--log-count N] [--log-days N] [--revision-count N] [--json]', get description() { return translate('component.helpView.readOrUpdateHistoryTrashActivityAndRevisionRetention'); } },
      { usage: 'pasted settings list|get|set [arguments] [--json]', get description() { return translate('component.helpView.inspectOrChangePersistedApplicationSettings'); } },
      { usage: 'pasted recording status|pause|resume [--json]', get description() { return translate('component.helpView.controlClipboardRecordingInTheRunningApp'); } },
      { usage: 'pasted queue status|start|stop|add|remove|order|paste|paste-all [arguments] [--json]', get description() { return translate('component.helpView.manageAndRunTheLiveCopyQueue'); } },
      { usage: 'pasted clear --yes [--json]', get description() { return translate('component.helpView.permanentlyRemoveUnpinnedUnprotectedClips'); } },
    ],
  },
  {
    get title() { return translate('component.helpView.clipActions'); },
    commands: [
      { usage: 'pasted clip get <id> [--json]', get description() { return translate('component.helpView.inspectOneClipAndItsMetadata'); } },
      { usage: 'pasted clip note <id> [--text TEXT | --clear | --stdin] [--json]', get description() { return translate('component.helpView.setOrClearAClipNote'); } },
      { usage: 'pasted clip revisions <id> [--limit N] [--offset N] [--json]', get description() { return translate('component.helpView.listRetainedClipRevisions'); } },
      { usage: 'pasted clip restore-revision <id> <revision-id> [--json]', get description() { return translate('component.helpView.restoreAnEarlierClipRevisionAndItsRecordedOrganization'); } },
      { usage: 'pasted clip provenance <id> [--json]', get description() { return translate('component.helpView.inspectTheTransformThatProducedTheCurrentClipContent'); } },
      { usage: 'pasted clip copy|paste <id> [--json]', get description() { return translate('component.helpView.copyOrPasteASavedClipThroughTheRunningApp'); } },
      { usage: 'pasted clip shortcut <id> <shortcut|none> [--json]', get description() { return translate('component.helpView.setOrClearAClipShortcutAndProtectAssignments'); } },
      { usage: 'pasted clip pin|unpin <id>... [--json]', get description() { return translate('component.helpView.setPinStateExplicitlyForOneOrMoreClips'); } },
      { usage: 'pasted clip order-pinned <id>... [--json]', get description() { return translate('component.helpView.replaceTheCompletePinnedClipOrder'); } },
      { usage: 'pasted clip protect|unprotect <id>... [--json]', get description() { return translate('component.helpView.setProtectionExplicitlyForOneOrMoreClips'); } },
      { usage: 'pasted clip trash|restore <id>... [--json]', get description() { return translate('component.helpView.moveClipsIntoOrOutOfTrash'); } },
      { usage: 'pasted clip restore-all [--json]', get description() { return translate('component.helpView.returnEveryTrashedClipToHistory'); } },
      { usage: 'pasted clip purge <id>... --yes [--json]', get description() { return translate('component.helpView.permanentlyDeleteUnprotectedClips'); } },
      { usage: 'pasted clip empty-trash --yes [--json]', get description() { return translate('component.helpView.permanentlyDeleteEveryUnprotectedClipInTrash'); } },
      { usage: 'pasted clip export [path] [--format json|csv]', get description() { return translate('component.helpView.exportClipsCurrentlyInHistoryForExternalAnalysis'); } },
      { usage: 'pasted clip import <path> [--format json|csv] [--json]', get description() { return translate('component.helpView.preflightAndMergeClipRecordsWhileSkippingDuplicates'); } },
      { usage: 'pasted clip assign <bin-id|none> <id>... [--json]', get description() { return translate('component.helpView.assignClipsToOneManualBinOrRemoveTheirManualBin'); } },
    ],
  },
  {
    get title() { return translate('component.helpView.binsAndTransforms'); },
    commands: [
      { usage: 'pasted bin list [--json]', get description() { return translate('component.helpView.listBinsCountsAndSavedOrdering'); } },
      { usage: 'pasted bin get <id> [--json]', get description() { return translate('component.helpView.inspectOneBinAndItsAttachedTransform'); } },
      { usage: 'pasted bin create --name NAME [options] [--json]', get description() { return translate('component.helpView.createAManualOrSmartBin'); } },
      { usage: 'pasted bin update <id> [options] [--json]', get description() { return translate('component.helpView.updateABinDefinition'); } },
      { usage: 'pasted bin duplicate <id> [--name NAME] [--json]', get description() { return translate('component.helpView.duplicateABinAndItsAttachedTransform'); } },
      { usage: 'pasted bin delete <id> [--disposition keep|trash|move] [--json]', get description() { return translate('component.helpView.deleteABinWithAnExplicitClipDisposition'); } },
      { usage: 'pasted bin clips <bin-id> [--json]', get description() { return translate('component.helpView.listABinSClipsInPersistentOrder'); } },
      { usage: 'pasted bin order <bin-id> <clip-id>... [--json]', get description() { return translate('component.helpView.replaceABinSCompleteSavedClipOrder'); } },
      { usage: 'pasted bin transform <id> <transform-ref|none> [--json]', get description() { return translate('component.helpView.setOrClearABinSDefaultTransform'); } },
      { usage: 'pasted bin shortcut <id> <shortcut|none> [--json]', get description() { return translate('component.helpView.setOrClearABinShortcut'); } },
      { usage: 'pasted bin protect <id> <on|off> [--json]', get description() { return translate('component.helpView.setInheritedProtectionForAManualBin'); } },
      { usage: 'pasted transform list [--json]', get description() { return translate('component.helpView.listSavedAndManuallyBuiltTransforms'); } },
      { usage: 'pasted transform get <ref> [--json]', get description() { return translate('component.helpView.inspectOneCanonicalTransformDefinition'); } },
      { usage: 'pasted transform plan [--intent TEXT | --stdin] [--sample TEXT] [--json]', get description() { return translate('component.helpView.draftATransformPlanFromNaturalLanguageIntent'); } },
      { usage: 'pasted transform test --plan-json JSON [--text TEXT | --stdin] [--json]', get description() { return translate('component.helpView.executeAnUnsavedTransformPlanWithoutChangingAClip'); } },
      { usage: 'pasted transform create --name NAME (--intent TEXT | --plan-json JSON | --steps-json JSON) [--json]', get description() { return translate('component.helpView.createAnIntentPlannedOrManuallyBuiltTransform'); } },
      { usage: 'pasted transform update <ref> [options] [--json]', get description() { return translate('component.helpView.updateATransformWithoutChangingItsStableReferenceOrAuthoringForm'); } },
      { usage: 'pasted transform duplicate <ref> [--name NAME] [--json]', get description() { return translate('component.helpView.duplicateATransformWithANewStableReference'); } },
      { usage: 'pasted transform delete <ref> [--json]', get description() { return translate('component.helpView.deleteATransformExistingClipRevisionsRemainUnchanged'); } },
      { usage: 'pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]', get description() { return translate('component.helpView.runATransformInPreviewModeOrApplyItToAClip'); } },
      { usage: 'pasted operation list [--json]', get description() { return translate('component.helpView.inspectBuiltInAndCustomOperations'); } },
      { usage: 'pasted operation get <ref> [--json]', get description() { return translate('component.helpView.inspectOneOperationDefinition'); } },
      { usage: 'pasted operation create --name NAME --type TYPE [options] [--json]', get description() { return translate('component.helpView.createAnOperation'); } },
      { usage: 'pasted operation update <ref> [options] [--json]', get description() { return translate('component.helpView.updateACustomOperation'); } },
      { usage: 'pasted operation duplicate <ref> [--name NAME] [--json]', get description() { return translate('component.helpView.duplicateAnOperationWithANewStableReference'); } },
      { usage: 'pasted operation delete <ref> [--json]', get description() { return translate('component.helpView.deleteACustomOperation'); } },
      { usage: 'pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]', get description() { return translate('component.helpView.runOneOperationThroughTheSharedExecutor'); } },
      { usage: 'pasted connection list [--json]', get description() { return translate('component.helpView.listConnectedIntelligenceProvidersInPriorityOrder'); } },
      { usage: 'pasted connection get <id> [--json]', get description() { return translate('component.helpView.inspectOneConnectionDefinition'); } },
      { usage: 'pasted connection detect [--json]', get description() { return translate('component.helpView.discoverSupportedLocalIntelligenceProviders'); } },
      { usage: 'pasted connection create --name NAME --provider KIND [options] [--json]', get description() { return translate('component.helpView.createAConnectionUsingCredentialReferencesOnly'); } },
      { usage: 'pasted connection update <id> [options] [--json]', get description() { return translate('component.helpView.updateOrEnableAConnection'); } },
      { usage: 'pasted connection delete <id> [--json]', get description() { return translate('component.helpView.deleteAConnectionDefinition'); } },
      { usage: 'pasted connection order <id>... [--json]', get description() { return translate('component.helpView.replaceConnectionPriorityOrder'); } },
    ],
  },
  {
    get title() { return translate('component.helpView.contentAnalysis'); },
    commands: [
      { usage: 'pasted analyzer run [--text TEXT | --clip ID | --stdin] [--policy POLICY] [--extract] [--json]', get description() { return translate('component.helpView.previewOneVersionedContentFreeSnapshotAcrossTheApplicableAnalysisPasses'); } },
      { usage: 'pasted registry list [--kind capture|inspector|extractor|classifier|suggestion|operation|transform] [--all] [--json]', get description() { return translate('component.helpView.inspectSharedLifecycleAndInputOutputContractsForProcessingAssets'); } },
      { usage: 'pasted registry enable|disable --kind extractor|classifier|operation --ref REF [--json]', get description() { return translate('component.helpView.changeTheSharedEnabledStateUsingAStableProcessingAssetReference'); } },
      { usage: 'pasted inspector list [--json]', get description() { return translate('component.helpView.listInspectorsContractsAndSystemAvailability'); } },
      { usage: 'pasted inspector get <ref> [--json]', get description() { return translate('component.helpView.inspectOneInspectorDefinition'); } },
      { usage: 'pasted inspector run [--text TEXT | --clip ID | --stdin] [--apply] [--json]', get description() { return translate('component.helpView.inspectContentFreeStructureAndLiveMediaMetadataOrPersistClipStructure'); } },
      { usage: 'pasted suggestion list [--json]', get description() { return translate('component.helpView.listSuggestionsAndTheirContracts'); } },
      { usage: 'pasted suggestion get <ref> [--json]', get description() { return translate('component.helpView.inspectOneSuggestionDefinition'); } },
      { usage: 'pasted suggestion run [--text TEXT | --clip ID | --stdin] [--json]', get description() { return translate('component.helpView.suggestSavedTransformsWithoutChangingContent'); } },
      { usage: 'pasted extractor list [--json]', get description() { return translate('component.helpView.listExtractorsContractsAndSystemAvailability'); } },
      { usage: 'pasted extractor get <ref> [--json]', get description() { return translate('component.helpView.inspectOneExtractorDefinition'); } },
      { usage: 'pasted extractor create (--recipe FILE | --prompt TEXT) [options] [--json]', get description() { return translate('component.helpView.createAnExtractor'); } },
      { usage: 'pasted extractor update <ref> [options] [--json]', get description() { return translate('component.helpView.updateAnExtractorDefinition'); } },
      { usage: 'pasted extractor propose --prompt TEXT [--connection ID] [--json]', get description() { return translate('component.helpView.draftAnExtractorRecipeWithAConnectedAi'); } },
      { usage: 'pasted extractor history <ref> [--json]', get description() { return translate('component.helpView.reviewLocalExtractorAuthoringHistory'); } },
      { usage: 'pasted extractor duplicate <ref> [--name NAME] [--json]', get description() { return translate('component.helpView.duplicateAnExtractorWithANewStableReference'); } },
      { usage: 'pasted extractor delete <ref> [--json]', get description() { return translate('component.helpView.deleteAnExtractorShippedDefaultsRemainRecoverable'); } },
      { usage: 'pasted extractor run <ref> (--clip ID | --file PATH) [--apply] [--json]', get description() { return translate('component.helpView.runAnExtractorInPreviewModeOrApplyItsOutputToA'); } },
      { usage: 'pasted extractor restore-defaults', get description() { return translate('component.helpView.restoreShippedExtractorSettings'); } },
      { usage: 'pasted type list [--all] [--json]', get description() { return translate('component.helpView.listRegisteredContentTypesAndTheirDisplayMetadata'); } },
      { usage: 'pasted type create --id ID --name NAME [--icon ICON] [--group GROUP] [--json]', get description() { return translate('component.helpView.createACustomContentTypeWithAStableId'); } },
      { usage: 'pasted type update <id> [options] [--json]', get description() { return translate('component.helpView.customizeAContentTypeSNameIconOrGroupWithoutChangingIts'); } },
      { usage: 'pasted type archive|restore <id>', get description() { return translate('component.helpView.archiveOrRestoreACustomContentTypeWhilePreservingHistoricalClips'); } },
      { usage: 'pasted type restore-defaults', get description() { return translate('component.helpView.restoreBuiltInContentTypeNamesIconsAndGroups'); } },
      { usage: 'pasted type group-list [--all] [--json]', get description() { return translate('component.helpView.listRegisteredContentTypeGroups'); } },
      { usage: 'pasted type group-create --id ID --name NAME [--order NUMBER]', get description() { return translate('component.helpView.createAReusableCustomContentTypeGroup'); } },
      { usage: 'pasted type group-update <id> [options] [--json]', get description() { return translate('component.helpView.renameOrReorderAContentTypeGroup'); } },
      { usage: 'pasted type group-archive|group-restore <id>', get description() { return translate('component.helpView.archiveAnEmptyCustomGroupOrRestoreIt'); } },
      { usage: 'pasted type group-delete <id>', get description() { return translate('component.helpView.permanentlyDeleteAnEmptyCustomGroup'); } },
      { usage: 'pasted classifier list [--json]', get description() { return translate('component.helpView.listClassifiersInEffectivePriorityOrder'); } },
      { usage: 'pasted classifier get <ref> [--json]', get description() { return translate('component.helpView.inspectOneClassifierDefinition'); } },
      { usage: 'pasted classifier create --name NAME --type TYPE --regex REGEX [--json]', get description() { return translate('component.helpView.createAClassifier'); } },
      { usage: 'pasted classifier update <ref> [options] [--json]', get description() { return translate('component.helpView.updateAClassifierDefinition'); } },
      { usage: 'pasted classifier duplicate <ref> [--name NAME] [--json]', get description() { return translate('component.helpView.duplicateAClassifierWithANewStableReference'); } },
      { usage: 'pasted classifier delete <ref> [--json]', get description() { return translate('component.helpView.deleteAClassifierShippedDefaultsRemainRecoverable'); } },
      { usage: 'pasted classifier run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]', get description() { return translate('component.helpView.runAClassifierInPreviewModeOrApplyItsMatchingContentType'); } },
      { usage: 'pasted classifier restore-defaults', get description() { return translate('component.helpView.restoreShippedClassifiersWithoutRemovingCustomEntries'); } },
      { usage: 'pasted classifier rescan --yes [--json]', get description() { return translate('component.helpView.explicitlyReclassifyExistingTextClipsWithTheCurrentEnabledClassifierOrder'); } },
    ],
  },
  {
    get title() { return translate('component.helpView.maintenance'); },
    commands: [
      { usage: 'pasted diagnostics [--json]', get description() { return translate('component.helpView.showInstallationSigningPathsAndRuntimeDetails'); } },
      { usage: 'pasted insights summary [--json]', get description() { return translate('component.helpView.summarizeClipTypesFileFormatsContentTypesSourcesAndDailyActivity'); } },
      { usage: 'pasted licenses [--json]', get description() { return translate('component.helpView.showTheBundledOpenSourceComponentInventoryAndLegalNotices'); } },
      { usage: 'pasted database location [--json]', get description() { return translate('component.helpView.showTheActiveSqliteDatabaseLocation'); } },
      { usage: 'pasted database protection [--json]', get description() { return translate('component.helpView.inspectVolumeEncryptionForTheActiveDatabase'); } },
      { usage: 'pasted database move <folder> [--json]', get description() { return translate('component.helpView.moveTheDatabaseSafelyAfterQuitting'); } },
      { usage: 'pasted database default [--json]', get description() { return translate('component.helpView.returnTheSqliteDatabaseToItsNativeDefaultLocation'); } },
      { usage: 'pasted transfer export <path.json> [--json]', get description() { return translate('component.helpView.exportHistoryAndOrganizationAsPortableJson'); } },
      { usage: 'pasted transfer inspect <path.json> [--json]', get description() { return translate('component.helpView.validateAndSummarizePortableJsonWithoutChangingSavedData'); } },
      { usage: 'pasted transfer import <path.json> [--json]', get description() { return translate('component.helpView.preflightAndMergeHistoryAndOrganizationByStableIdentityAndContentHash'); } },
      { usage: 'pasted backup create <path.pastedbackup> [--json]', get description() { return translate('component.helpView.createAValidatedSnapshotOfEveryDurableStateStore'); } },
      { usage: 'pasted backup inspect <path.pastedbackup> [--json]', get description() { return translate('component.helpView.validateAFullBackupAndInspectItsManifestWithoutRestoringIt'); } },
      { usage: 'pasted backup restore <path.pastedbackup> --yes [--json]', get description() { return translate('component.helpView.replaceTheCurrentStateAfterCreatingACompleteRecoveryBackup'); } },
      { usage: 'pasted ocr status [--json]', get description() { return translate('component.helpView.inspectOcrBackfillProgress'); } },
      { usage: 'pasted ocr scan [--clip ID] [--json]', get description() { return translate('component.helpView.processEligibleImagesOrRescanOneImageClip'); } },
      { usage: 'pasted ocr retry [--json]', get description() { return translate('component.helpView.resetFailedOcrAttemptsAndProcessThemAgain'); } },
      { usage: 'pasted ocr cancel [--json]', get description() { return translate('component.helpView.cancelOcrWorkInTheRunningApp'); } },
      { usage: 'pasted reset --yes [--json]', get description() { return translate('component.helpView.resetAllDataAndPreferencesThisIsDestructive'); } },
    ],
  },
  {
    get title() { return translate('destination.activity'); },
    commands: [
      { usage: 'pasted activity list [--limit N|--all] [--offset N] [--category VALUE] [--severity VALUE] [--event NAME] [--json]', get description() { return translate('component.helpView.listOrFilterABoundedPageOfRetainedActivityEntries'); } },
      { usage: 'pasted activity export [path] [--format json|csv]', get description() { return translate('component.helpView.exportAllRetainedActivityEntriesForReporting'); } },
      { usage: 'pasted activity import <path> [--format json|csv] [--json]', get description() { return translate('component.helpView.mergeInertActivityRecordsWithoutReplayingTheirActions'); } },
      { usage: 'pasted activity clear --yes [--json]', get description() { return translate('component.helpView.permanentlyRemoveEveryRetainedActivityEntry'); } },
    ],
  },
] as const;

interface HelpTopicDefinition {
  id: HelpTopic;
  label: string;
  icon: LucideIcon;
  iconClassName: string;
}

const HELP_TOPICS: HelpTopicDefinition[] = [
  { id: 'getting-started', get label() { return translate('component.helpView.gettingStarted'); }, icon: BookOpen, iconClassName: 'theme-status-info-text' },
  { id: 'shortcuts-hud', get label() { return translate('component.helpView.shortcutsAndHud'); }, icon: Keyboard, iconClassName: 'theme-status-success-text' },
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
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Terminal className="w-5 h-5 theme-status-info-text" />
                  <span>{translate('component.helpView.terminalCliCommand', { command: 'pasted' })}</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  {translate('component.helpView.theStandaloneNativeCommandLineToolCanPipeDataIntoClipboardHistory')}
                </p>
              </div>

              {/* PATH Installation Box */}
              <div className="theme-status-info p-4 rounded-xl border space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2 text-xs font-bold">
                    <Download className="w-4 h-4" />
                    <span>{translate('component.helpView.installCliToPath')}</span>
                  </div>
                  <button
                    onClick={handleInstallCli}
                    className="theme-primary-button ui-control-radius flex items-center space-x-1.5 px-3 py-1.5 border text-xs font-bold transition-colors cursor-pointer shadow-sm"
                  >
                    <Download className="w-3.5 h-3.5" />
                    <span>{translate('component.helpView.value1ClickSymlinkToLocalBin')}</span>
                  </button>
                </div>

                <div className="theme-text-main space-y-2 text-xs">
                  <p className="font-semibold theme-title">{translate('component.helpView.manualPathSetup')}</p>
                  <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">{translate('component.helpView.symlinkInUsrLocalBin')}</span>
                        <button
                          type="button"
                          onClick={() => handleCopyCode(CLI_SYMLINK_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title={translate('component.helpView.copyCommand')}
                        >
                          {copiedCmd === CLI_SYMLINK_COMMAND ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_SYMLINK_COMMAND}</code>
                    </div>

                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">{translate('component.helpView.shellAlias')}</span>
                        <button
                          type="button"
                          onClick={() => handleCopyCode(CLI_ALIAS_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title={translate('component.helpView.copyAlias')}
                        >
                          {copiedCmd === CLI_ALIAS_COMMAND ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_ALIAS_COMMAND}</code>
                    </div>
                  </div>
                </div>
              </div>

              <div className="space-y-3">
                <div>
                  <h4 className="theme-title text-sm font-bold">{translate('component.helpView.commandReference')}</h4>
                  <p className="theme-text-muted mt-1 text-xs">
                    {translate('component.helpView.commandReferenceDescription', { flag: '--json' })}</p>
                </div>
                <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
                  {CLI_COMMAND_GROUPS.map((group) => (
                    <section key={group.title} className="theme-panel overflow-hidden rounded-xl border">
                      <h5 className="theme-section-label theme-divider border-b px-4 py-3 text-[11px] font-bold uppercase tracking-[0.12em]">
                        {group.title}
                      </h5>
                      <div className="theme-divide divide-y">
                        {group.commands.map((command) => (
                          <div key={command.usage} className="flex items-start gap-3 px-4 py-3">
                            <div className="min-w-0 flex-1">
                              <code className="selectable-text theme-status-info-text block select-text break-all font-mono text-[11px] font-semibold">
                                {command.usage}
                              </code>
                              <p className="theme-text-muted mt-1 text-xs leading-relaxed">{command.description}</p>
                            </div>
                            <button
                              type="button"
                              onClick={() => handleCopyCode(command.usage)}
                              className="theme-icon-button shrink-0 rounded border p-1.5"
                              title={translate('component.helpView.copyCommand')}
                            >
                              {copiedCmd === command.usage ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                            </button>
                          </div>
                        ))}
                      </div>
                    </section>
                  ))}
                </div>
              </div>
            </div>
          )}

          {activeTopic === 'shortcuts-hud' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Keyboard className="w-5 h-5 theme-status-success-text" />
                  <span>{translate('component.helpView.shortcutsAndHud')}</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  {translate('component.helpView.useTheDefaultShortcutsBelowOrChangeAndDisableThemUnderSettings')}
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
                    {translate('component.helpView.openHudShortcutDescription', { shortcut: '⌥ Shift V' })}</p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-info-text flex items-center space-x-2 text-xs font-bold">
                    <Zap className="w-4 h-4" />
                    <span>{translate('component.helpView.hudNumberKeys19')}</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    {translate('component.helpView.hudNumberShortcutDescription', { start: 1, end: 9 })}</p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-success-text flex items-center space-x-2 text-xs font-bold">
                    <Info className="w-4 h-4" />
                    <span>{translate('component.helpView.escapeKeyDismiss')}</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    {translate('component.helpView.dismissHudShortcutDescription', { key: translate('component.helpView.esc') })}</p>
                </div>
              </div>
            </div>
          )}

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
                    <span>{translate('component.helpView.clipShortcuts')}</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    {translate('component.helpView.clipShortcutDescription')}
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
