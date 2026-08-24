import { CircleAlert, CircleCheck, RotateCcw, ScanText } from 'lucide-react';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { ConfirmationDialog } from './ConfirmationDialog';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { RegistryPanelFooter } from './RegistryPanelFooter';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { translate } from '../localization/runtime';
import { ExtractorAuthoringHistoryDialog } from './ExtractorAuthoringHistoryDialog';
import { ExtractorAiAuthoringPanel } from './ExtractorAiAuthoringPanel';
import { ExtractorAiSetupPanel } from './ExtractorAiSetupPanel';
import { ExtractorRecipeEditor } from './ExtractorRecipeEditor';
import { ExtractorRegistryPanel } from './ExtractorRegistryPanel';
import { useContentExtractorManager } from '../hooks/useContentExtractorManager';

export function ContentExtractorManagerDialog({
  isOpen,
  onClose,
  onChanged,
  onOpenIntelligence,
  ocrEnabled,
  transcriptionsEnabled,
}: {
  isOpen: boolean;
  onClose: () => void;
  onChanged?: () => void;
  onOpenIntelligence?: () => void;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
}) {
  const {
    authoringHistory,
    authoringPrompt,
    availabilityLabel,
    availabilityTitle,
    beginNew,
    cancelDraft,
    chooseResource,
    chooseStepExecutable,
    confirmation,
    defaults,
    diagnostic,
    differsFromDefaults,
    draft,
    duplicate,
    generateRecipe,
    generating,
    hasIntelligence,
    isDirty,
    loading,
    openAuthoringHistory,
    recipeCanSave,
    recipeDraft,
    repairRecipe,
    repairing,
    aiSetup,
    remove,
    resetDraft,
    restoreAll,
    runtimeConfigurationChanged,
    runtimeLoadingId,
    save,
    saving,
    selected,
    selectedId,
    selectExtractor,
    setAuthoringHistory,
    setAuthoringPrompt,
    setConfirmation,
    setDraft,
    setRecipeDraft,
    setupGuidance,
    testOutcome,
    testRecipe,
    testing,
    toggle,
    visibleExtractors,
  } = useContentExtractorManager({
    isOpen,
    onChanged,
    ocrEnabled,
    transcriptionsEnabled,
  });
  return <><AppDialog
    isOpen={isOpen}
    onClose={onClose}
    labelledBy="extractor-manager-title"
    isDirty={isDirty}
    discardMessage={translate('component.contentExtractorManagerDialog.discardExtractorChanges')}
    panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
  >
    {({ requestClose }) => <>
      <AppDialogHeader onClose={requestClose} className="shrink-0">
        <AppDialogHeading
          id="extractor-manager-title"
          title={translate('component.contentExtractorManagerDialog.extractors')}
          description={translate('component.contentExtractorManagerDialog.createSearchableRepresentationsFromClipContentTheLowestPriorityNumberRunsFirst')}
          icon={<ScanText />}
        />
      </AppDialogHeader>
      <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
        <ExtractorRegistryPanel
          extractors={visibleExtractors}
          selectedId={selectedId}
          selected={selected}
          isDirty={isDirty}
          saving={saving}
          loading={loading}
          onNew={beginNew}
          onSelect={selectExtractor}
          onToggle={toggle}
          onDuplicate={duplicate}
          onRemove={remove}
        />
        <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
          <RegistryPanelHeader
            title={translate('component.contentExtractorManagerDialog.extractorSettings')}
            actions={<span
              title={availabilityTitle}
              className={`${selectedId !== 'new' && !runtimeConfigurationChanged && selected?.isAvailable
                ? 'theme-status-success-text'
                : selectedId !== 'new' && !runtimeConfigurationChanged
                  ? 'theme-status-warning-text'
                  : 'theme-text-muted'} flex min-w-0 max-w-[70%] shrink items-center gap-1.5 text-[10px] font-semibold`}
            >
              {selectedId !== 'new' && !runtimeConfigurationChanged && selected?.isAvailable
                ? <CircleCheck className="h-3.5 w-3.5 shrink-0" />
                : <CircleAlert className="h-3.5 w-3.5 shrink-0" />}
              <span className="truncate">{availabilityLabel}</span>
            </span>}
          />
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_110px_auto] @md:items-end">
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.name !== defaults?.name}>{translate('common.name')}</ModifiedFieldLabel>
                <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
              </label>
              <label className="flex h-9 items-center gap-2 px-1">
                <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.enabled !== defaults?.enabled}>{translate('common.enabled')}</ModifiedFieldLabel>
              </label>
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.priority !== defaults?.priority}>{translate('common.priority')}</ModifiedFieldLabel>
                <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono" />
              </label>
            </div>
            <label className="block space-y-1">
              <ModifiedFieldLabel modified={selectedId !== 'new' && draft.description !== defaults?.description}>{translate('common.description')}</ModifiedFieldLabel>
              <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
            </label>
            <ExtractorAiAuthoringPanel
              isNew={selectedId === 'new'} prompt={authoringPrompt} generating={generating}
              hasIntelligence={hasIntelligence}
              onPromptChange={setAuthoringPrompt} onGenerate={() => void generateRecipe()}
              onOpenIntelligence={onOpenIntelligence}
            />
            <ExtractorAiSetupPanel
              visible={aiSetup.visible}
              hasIntelligence={hasIntelligence}
              repairing={repairing}
              guidanceIncomplete={aiSetup.guidanceIncomplete}
              diagnostic={diagnostic}
              setupGuidance={setupGuidance}
              onRepair={() => void repairRecipe()}
              onOpenIntelligence={onOpenIntelligence}
            />
            <ExtractorRecipeEditor
              recipe={recipeDraft}
              onChange={setRecipeDraft}
              onChooseExecutable={chooseStepExecutable}
              onChooseResource={chooseResource}
              onTest={testRecipe}
              testing={testing}
              testOutcome={testOutcome}
              canSave={recipeCanSave}
            />
            <details className="theme-subtle-surface rounded-xl border p-3 text-[10px]">
              <summary className="theme-text-muted cursor-pointer font-semibold">{translate('common.technicalDetails')}</summary>
              <dl className="mt-3 grid grid-cols-[110px_minmax(0,1fr)] gap-x-3 gap-y-2">
                <dt className="theme-text-muted">{translate('common.stableReference')}</dt><dd className="truncate font-mono">{selected?.stableRef ?? translate('component.contentExtractorManagerDialog.assignedWhenSaved')}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.recipeVersion')}</dt><dd className="font-mono">{recipeDraft.definitionVersion}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.revision')}</dt><dd>{selected?.revision ?? 1}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.runtimeVersion')}</dt><dd>{runtimeLoadingId === selected?.id
                  ? translate('component.contentExtractorManagerDialog.loadingRuntime')
                  : selected?.runtime.version ?? translate('component.contentExtractorManagerDialog.unavailable')}</dd>
              </dl>
              {selected && <AppDialogButton type="button" className="mt-3" onClick={() => void openAuthoringHistory()}>{translate('component.contentExtractorManagerDialog.viewAuthoringHistory')}</AppDialogButton>}
            </details>
          </div>
          <RegistryPanelFooter>
            <div>
              {selected?.isBuiltin && <AppDialogButton onClick={resetDraft} disabled={!differsFromDefaults || saving}><RotateCcw className="h-3.5 w-3.5" /> {translate('common.resetToDefault')}</AppDialogButton>}
            </div>
            <div className="flex items-center gap-2">
              <AppDialogButton onClick={cancelDraft} disabled={selectedId !== 'new' && !isDirty}>{translate('common.cancel')}</AppDialogButton>
              <AppDialogButton variant="primary" onClick={() => void save()} disabled={selectedId === null || saving || !recipeCanSave || (selectedId !== 'new' && !isDirty)}><SaveButtonContent isSaving={saving} /></AppDialogButton>
            </div>
          </RegistryPanelFooter>
        </section>
      </AppDialogBody>
      <AppDialogFooter align="between" className="shrink-0">
        <AppDialogButton onClick={restoreAll} disabled={saving}>
          <RotateCcw className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.reset')}
        </AppDialogButton>
        <AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton>
      </AppDialogFooter>
    </>}
  </AppDialog>
  <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  <ExtractorAuthoringHistoryDialog
    sessions={authoringHistory}
    onClose={() => setAuthoringHistory(null)}
  />
  </>;
}
