import { Copy, Plus, Radar, RotateCcw, Shapes, Trash2 } from 'lucide-react';
import { useState } from 'react';

import { useClassifierManager } from '../hooks/useClassifierManager';
import { translate } from '../localization/runtime';
import { localizedBuiltinDescription, localizedBuiltinName, localizedContentTypeGroupLabel } from '../localization/presentation';
import type { ClipContentType } from '../types';
import { contentTypeLabel } from '../utils/contentTypes';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
  SaveButtonContent,
} from './AppDialogLayout';
import { ConfirmationDialog } from './ConfirmationDialog';
import { ConnectedMenuAction } from './ConnectedMenuAction';
import { ContentTypeIcon } from './ContentTypeIcon';
import { ContentTypeManagerDialog } from './ContentTypeManagerDialog';
import { useContentTypes } from './ContentTypeProvider';
import { MenuSelect } from './MenuSelect';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelFooter } from './RegistryPanelFooter';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { SettingsSwitch } from './SettingsSwitch';

export function ClassifierManagerDialog({
  isOpen,
  onClose,
}: {
  isOpen: boolean;
  onClose: () => void;
}) {
  const {
    definitions: contentTypes,
    groups: contentTypeGroups,
    refresh: refreshContentTypes,
    refreshGroups,
  } = useContentTypes();
  const [isTypeManagerOpen, setIsTypeManagerOpen] = useState(false);
  const {
    beginNew,
    cancelDraft,
    classifiers,
    confirmation,
    draft,
    duplicate,
    hasModifiedFields,
    isEditorDirty,
    modified,
    patternsText,
    remove,
    resetSelectedDraft,
    restoreDefaults,
    sample,
    sampleMatched,
    save,
    saving,
    selected,
    selectedId,
    selectClassifier,
    setConfirmation,
    setDraft,
    setPatternsText,
    setSample,
    setSampleMatched,
    test,
    toggle,
    togglingId,
  } = useClassifierManager({ isOpen, refreshContentTypes, refreshGroups });

  return <>
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="classifier-manager-title"
      isDirty={isEditorDirty}
      discardMessage={translate('component.settingsAnalysisPanel.discardClassifierChanges')}
      panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} className="shrink-0">
          <AppDialogHeading
            id="classifier-manager-title"
            title={translate('component.settingsAnalysisPanel.classifiers')}
            description={translate('component.settingsAnalysisPanel.manageHowCopiedTextIsClassifiedTheLowestPriorityNumberRunsFirst')}
            icon={<Radar />}
          />
        </AppDialogHeader>
        <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
          <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
            <RegistryPanelHeader
              title={translate('component.settingsAnalysisPanel.classifiers')}
              actions={<AppDialogButton onClick={beginNew} className="h-7 min-h-7 px-2.5">
                <Plus className="h-3 w-3" /> {translate('common.new')}
              </AppDialogButton>}
            />
            <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
              {classifiers.map((classifier) => {
                const displayName = localizedBuiltinName('classifier', classifier.stable_ref, classifier.name, classifier.is_builtin, classifier.defaults?.name);
                const displayDescription = localizedBuiltinDescription('classifier', classifier.stable_ref, classifier.description, classifier.is_builtin, classifier.defaults?.description);
                return <RegistryListItem
                  key={classifier.id}
                  selected={selectedId === classifier.id}
                  onSelect={() => selectClassifier(classifier.id)}
                  icon={<ContentTypeIcon type={classifier.content_type as ClipContentType} className="h-4 w-4" />}
                  title={displayName}
                  subtitle={displayDescription}
                  trailing={<SettingsSwitch
                    checked={classifier.enabled}
                    label={displayName}
                    busy={togglingId === classifier.id}
                    onClick={() => toggle(classifier)}
                  />}
                />;
              })}
            </div>
            <RegistryPanelFooter align="end">
              <AppDialogButton onClick={() => void duplicate()} disabled={!selected || isEditorDirty || saving} title={isEditorDirty ? translate('component.settingsAnalysisPanel.saveOrCancelChangesBeforeDuplicating') : undefined}><Copy className="h-3.5 w-3.5" /> {translate('common.duplicate')}</AppDialogButton>
              <AppDialogButton variant="danger" onClick={remove} disabled={!selected || saving}><Trash2 className="h-3.5 w-3.5" /> {translate('component.settingsAnalysisPanel.delete')}</AppDialogButton>
            </RegistryPanelFooter>
          </section>
          <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
            <RegistryPanelHeader title={translate('component.settingsAnalysisPanel.classifierSettings')} />
            <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-4">
              <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_minmax(270px,0.8fr)]">
                <label className={`space-y-1 ${modified.name ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.name}>{translate('common.name')}</ModifiedFieldLabel>
                  <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
                </label>
                <div className={`space-y-1 ${modified.content_type ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.content_type}>{translate('component.settingsAnalysisPanel.contentType')}</ModifiedFieldLabel>
                  <ConnectedMenuAction
                    className="w-full"
                    groupLabel={translate('component.settingsAnalysisPanel.classifierContentType')}
                    actionLabel={translate('component.settingsAnalysisPanel.manageContentTypes')}
                    action={<><Shapes className="h-3.5 w-3.5" aria-hidden="true" /><span>{translate('component.settingsAnalysisPanel.manage')}</span></>}
                    onAction={() => setIsTypeManagerOpen(true)}
                  >
                    <MenuSelect
                      value={draft.content_type}
                      onChange={(content_type) => setDraft({ ...draft, content_type })}
                      label={translate('component.settingsAnalysisPanel.classifierContentType')}
                      leadingIcon={<ContentTypeIcon type={draft.content_type as ClipContentType} className="h-4 w-4" />}
                      options={contentTypes.map((type) => ({
                        value: type.id,
                        label: contentTypeLabel(type.id),
                        group: (() => {
                          const group = contentTypeGroups.find(({ id }) => id === type.group);
                          return group ? localizedContentTypeGroupLabel(group.id, group.label, group.isBuiltin, group.defaults?.label) : type.group;
                        })(),
                        disabled: type.isArchived,
                        icon: <ContentTypeIcon type={type.id as ClipContentType} className="h-4 w-4" />,
                      }))}
                      className="min-w-0 flex-1"
                    />
                  </ConnectedMenuAction>
                </div>
              </div>
              <label className={`block space-y-1 ${modified.description ? 'settings-field-modified' : ''}`}>
                <ModifiedFieldLabel modified={modified.description}>{translate('common.description')}</ModifiedFieldLabel>
                <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
              </label>
              <div className="grid grid-cols-1 items-end gap-3 @md:grid-cols-[110px_minmax(180px,1fr)_auto]">
                <label className={`space-y-1 ${modified.priority ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.priority}>{translate('common.priority')}</ModifiedFieldLabel>
                  <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono" />
                </label>
                <label className={`space-y-1 ${modified.validator ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.validator}>{translate('component.settingsAnalysisPanel.validation')}</ModifiedFieldLabel>
                  <MenuSelect
                    value={draft.validator ?? ''}
                    onChange={(validator) => setDraft({ ...draft, validator: validator || null })}
                    options={[
                      { value: '', get label() { return translate('component.settingsAnalysisPanel.regexOnly'); } },
                      { value: 'luhn', get label() { return translate('component.settingsAnalysisPanel.cardChecksum'); } },
                      { value: 'iban', get label() { return translate('component.settingsAnalysisPanel.ibanChecksum'); } },
                      { value: 'ip', get label() { return translate('component.settingsAnalysisPanel.ipParser'); } },
                      { value: 'phone', get label() { return translate('component.settingsAnalysisPanel.phoneGuardrails'); } },
                      { value: 'env_block', get label() { return translate('component.settingsAnalysisPanel.environmentBlock'); } },
                      { value: 'prose', get label() { return translate('component.settingsAnalysisPanel.proseGuardrails'); } },
                    ]}
                    label={translate('component.settingsAnalysisPanel.semanticValidation')}
                    className="w-full"
                  />
                </label>
                <label className={`flex min-h-9 items-center gap-2 ${modified.enabled ? 'settings-field-modified' : ''}`}>
                  <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
                  <ModifiedFieldLabel modified={modified.enabled}>{translate('common.enabled')}</ModifiedFieldLabel>
                </label>
              </div>
              <label className={`block space-y-1 ${modified.patterns ? 'settings-field-modified' : ''}`}>
                <ModifiedFieldLabel modified={modified.patterns}>{translate('component.settingsAnalysisPanel.regularExpressions')} <span className="font-normal">{translate('component.settingsAnalysisPanel.onePerLineAnyMayMatch')}</span></ModifiedFieldLabel>
                <textarea dir="auto" value={patternsText} onChange={(event) => setPatternsText(event.target.value)} spellCheck={false} className="theme-input ui-field-radius min-h-32 w-full resize-y border px-3 py-2 font-mono text-[11px] leading-relaxed" />
              </label>
              {draft.validator && <div className="theme-status-info rounded-lg border px-3 py-2 text-[10px]">
                {translate('component.settingsAnalysisPanel.candidatesAlsoPassTheBuiltIn')} <strong>{draft.validator}</strong>{translate('component.settingsAnalysisPanel.validatorToReduceFalsePositives')}
              </div>}
              <div className="theme-divider grid grid-cols-[minmax(0,1fr)_auto] gap-2 border-t pt-3">
                <input value={sample} onChange={(event) => { setSample(event.target.value); setSampleMatched(null); }} placeholder={translate('component.settingsAnalysisPanel.trySampleText')} className="theme-input ui-field-radius border px-3 py-2 font-mono" />
                <AppDialogButton onClick={() => void test()} className="h-auto min-h-9">{translate('component.settingsAnalysisPanel.test')}</AppDialogButton>
              </div>
              {sampleMatched !== null && <div className={sampleMatched ? 'theme-status-success-text' : 'theme-status-danger-text'}>
                {sampleMatched ? translate('component.settingsAnalysisPanel.matchesThisClassifier') : translate('component.settingsAnalysisPanel.doesNotMatchThisClassifier')}
              </div>}
            </div>
            <RegistryPanelFooter>
              <div>
                {selected?.is_builtin && <AppDialogButton onClick={resetSelectedDraft} disabled={!hasModifiedFields || saving}><RotateCcw className="h-3.5 w-3.5" /> {translate('common.resetToDefault')}</AppDialogButton>}
              </div>
              <div className="flex items-center gap-2">
                <AppDialogButton onClick={cancelDraft} disabled={selectedId !== 'new' && !isEditorDirty}>{translate('common.cancel')}</AppDialogButton>
                <AppDialogButton variant="primary" onClick={() => void save()} disabled={saving || (selectedId !== 'new' && !isEditorDirty)}><SaveButtonContent isSaving={saving} /></AppDialogButton>
              </div>
            </RegistryPanelFooter>
          </section>
        </AppDialogBody>
        <AppDialogFooter align="between" className="shrink-0">
          <AppDialogButton onClick={restoreDefaults} disabled={saving}>
            <RotateCcw className="h-3.5 w-3.5" /> {translate('common.resetWithEllipsis')}
          </AppDialogButton>
          <AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
    <ContentTypeManagerDialog isOpen={isTypeManagerOpen} onClose={() => setIsTypeManagerOpen(false)} />
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  </>;
}
