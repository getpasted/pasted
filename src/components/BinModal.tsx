import React from 'react';
import { Folder, Plus, Minus } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { detectDesktopPlatform } from '../utils/platform';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { AnchoredMenu } from './AnchoredMenu';
import { MenuSelect } from './MenuSelect';
import { useContentTypes } from './ContentTypeProvider';
import { translate } from '../localization/runtime';
import { localizedContentTypeGroupLabel } from '../localization/presentation';
import { contentTypeLabel } from '../utils/contentTypes';
import { SettingsSwitch } from './SettingsSwitch';
import { SmartConditionTargetSelect, SmartConditionValueInput } from './BinModalSmartConditionInputs';
import {
  BIN_EMOJI_OPTIONS,
  COLOR_PALETTE,
  STRUCTURAL_CLIP_TYPES,
  emojiLabel,
  type SmartBinFeatures,
  type SmartConditionRow,
  type SmartConditionTarget,
  type SmartTargetSection,
} from './binModalModel';
import { useBinModalForm } from '../hooks/useBinModalForm';

interface BinModalProps {
  isOpen: boolean;
  editingBin?: Bin | null;
  features: SmartBinFeatures;
  fileFormats: string[];
  sources: string[];
  onClose: () => void;
  onRefreshBins: () => void;
}

export const BinModal: React.FC<BinModalProps> = ({
  isOpen,
  editingBin,
  features,
  fileFormats,
  sources,
  onClose,
  onRefreshBins,
}) => {
  const { definitions: contentTypes, groups: contentTypeGroups } = useContentTypes();
  const form = useBinModalForm({
    isOpen,
    editingBin,
    features,
    fileFormats,
    contentTypes,
    onClose,
    onRefreshBins,
  });
  const {
    modalTab, setModalTab,
    name, setName,
    selectedColor, setSelectedColor,
    icon, setIcon,
    isEmojiMenuOpen, setIsEmojiMenuOpen, emojiTriggerRef,
    errors, setErrors,
    installedApps,
    transforms, transformRef, setTransformRef,
    protectClips, setProtectClips,
    concealClips, setConcealClips,
    conditions, matchCondition, setMatchCondition,
    addCondition: handleAddCondition,
    removeCondition: handleRemoveCondition,
    updateCondition: handleUpdateCondition,
    submit: handleSubmit,
    isDirty,
  } = form;
  const desktopPlatform = detectDesktopPlatform();

  if (!isOpen) return null;

  const activeContentTypes = contentTypes.filter((type) => (
    !type.isArchived && !STRUCTURAL_CLIP_TYPES.has(type.id)
  ));
  const targetLabels: Record<SmartConditionTarget, string> = {
    clip_type: translate('component.binModal.clipType'),
    file_format: translate('component.binModal.fileFormat'),
    source: translate('component.binModal.source'),
    content_type: translate('component.binModal.contentType2'),
    origin_kind: translate('component.binModal.captureMethod'),
    contains: translate('component.binModal.textContent'),
    file_extension: translate('component.binModal.fileExtension'),
    file_path: translate('component.binModal.filePath'),
  };
  const targetSectionsFor = (condition: SmartConditionRow): SmartTargetSection[] => {
    const contentTypeChoices = activeContentTypes.map((type) => ({
      value: type.id,
      label: contentTypeLabel(type.id),
      group: (() => {
        const group = contentTypeGroups.find(({ id }) => id === type.group);
        return group ? localizedContentTypeGroupLabel(group.id, group.label, group.isBuiltin, group.defaults?.label) : type.group;
      })(),
    }));
    const formatChoices = fileFormats.map((format) => ({ value: format, label: format.toUpperCase() }));
    const sourceChoices = [...new Set([
      ...(condition.target === 'source' && condition.value ? [condition.value] : []),
      ...sources,
      ...installedApps,
    ])].map((source) => ({ value: source, label: source }));
    return [
      ...(features.clipTypes || condition.target === 'clip_type' ? [{
        target: 'clip_type' as const,
        label: targetLabels.clip_type,
        choices: [
          { value: 'text', label: translate('component.analyticsView.text'), disabled: !features.clipTypes },
          { value: 'image', label: translate('component.analyticsView.image'), disabled: !features.clipTypes },
          { value: 'file', label: translate('component.analyticsView.files'), disabled: !features.clipTypes },
        ],
      }] : []),
      ...(features.types || condition.target === 'content_type' ? [{
        target: 'content_type' as const,
        label: targetLabels.content_type,
        choices: contentTypeChoices.length > 0
          ? contentTypeChoices.map((choice) => ({ ...choice, disabled: !features.types }))
          : [{ value: condition.value, label: contentTypeLabel(condition.value), disabled: true }],
      }] : []),
      ...(features.fileFormats || condition.target === 'file_format' ? [{
        target: 'file_format' as const,
        label: targetLabels.file_format,
        choices: formatChoices.length > 0
          ? formatChoices.map((choice) => ({ ...choice, disabled: !features.fileFormats }))
          : [{ value: condition.value, label: condition.value.toUpperCase() || translate('component.binModal.noDetectedFileFormats'), disabled: true }],
      }] : []),
      ...(features.sources || condition.target === 'source' ? [{
        target: 'source' as const,
        label: targetLabels.source,
        choices: sourceChoices.length > 0
          ? sourceChoices.map((choice) => ({ ...choice, disabled: !features.sources }))
          : [{ value: condition.value, label: condition.value || translate('component.binModal.noDetectedApps'), disabled: true }],
      }] : []),
    ];
  };
  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="bin-modal-title"
      isDirty={isDirty}
      panelClassName="bin-modal-card theme-panel w-full max-w-2xl max-h-[90vh] border shadow-2xl overflow-hidden flex flex-col font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading id="bin-modal-title" title={editingBin ? translate('component.binModal.editBin') : translate('component.binModal.newBin')} description={translate('component.binModal.chooseHowClipsEnterThisBinAndWhatHappensNext')} icon={<Folder />} />
        </AppDialogHeader>
        <form onSubmit={handleSubmit} className="flex min-h-0 flex-1 flex-col">
          <AppDialogBody className="space-y-4 text-xs">
            <div className="flex justify-center">
              <div className="flex theme-surface p-1 rounded-xl border space-x-1">
                <button
                  type="button"
                  onClick={() => setModalTab('bin')}
                  className={`settings-tab px-4 py-1.5 rounded-lg text-xs font-semibold transition-none border border-transparent ${modalTab === 'bin' ? 'is-active' : ''}`}
                >
                  {translate('component.binModal.manual')}
                </button>
                <button
                  type="button"
                  onClick={() => setModalTab('smart')}
                  className={`settings-tab px-4 py-1.5 rounded-lg text-xs font-semibold transition-none border border-transparent ${modalTab === 'smart' ? 'is-active' : ''}`}
                >
                  {translate('component.binModal.smart')}
                </button>
              </div>
            </div>
          {/* Name Field */}
          <div className="flex items-center space-x-3">
            <label className={`w-20 text-end font-semibold flex-shrink-0 ${errors.name ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>{translate('common.name')}</label>
            <input
              type="text"
              placeholder={translate('component.binModal.eGCodeSnippetsSafariClips')}
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                if (errors.name) setErrors((prev) => ({ ...prev, name: false }));
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleSubmit(e);
                }
              }}
              className={`flex-1 theme-input ui-field-radius border px-3 py-2 text-xs focus:outline-none font-medium transition-colors ${errors.name ? 'form-field-error' : 'form-field-valid'}`}
              autoFocus
            />
          </div>

          {/* Color Palette Picker Row */}
          <div className="flex items-center space-x-3">
            <label className={`w-20 text-end font-semibold flex-shrink-0 ${errors.color ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>{translate('component.binModal.color')}</label>
            <div className={`flex items-center space-x-2 p-1 rounded-xl border border-transparent transition-colors ${errors.color ? 'form-field-error' : ''}`}>
              {COLOR_PALETTE.map((c) => (
                <button
                  key={c.hex}
                  type="button"
                  onClick={() => {
                    setSelectedColor(c.hex);
                    if (errors.color) setErrors((prev) => ({ ...prev, color: false }));
                  }}
                  style={{ backgroundColor: c.hex === 'default' ? 'var(--text-main)' : c.hex }}
                  className={`w-5 h-5 rounded-full border border-transparent transition-transform ${
                    selectedColor === c.hex
                      ? 'bin-color-selected scale-110'
                      : 'opacity-80 hover:opacity-100'
                  }`}
                  aria-label={translate('component.binModal.labelBinText', { label: c.label })}
                  title={c.label}
                />
              ))}
            </div>
          </div>

          {/* Single Emoji Icon Selector */}
          <div className="flex items-center space-x-3">
            <label className={`w-20 text-end font-semibold flex-shrink-0 ${errors.icon ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>{translate('component.binModal.icon')}</label>
            <div className="flex-1 flex items-center space-x-2.5">
              {desktopPlatform === 'macos' ? (
                <input
                  type="text"
                  value={formatEmojiIcon(icon)}
                  onChange={(event) => {
                    const value = event.target.value;
                    if (!value) return;
                    try {
                      const IntlWithSegmenter = Intl as typeof Intl & {
                        Segmenter?: new (locale?: string, options?: { granularity: string }) => {
                          segment: (input: string) => Iterable<{ segment: string }>;
                        };
                      };
                      if (typeof IntlWithSegmenter.Segmenter === 'function') {
                        const segmenter = new IntlWithSegmenter.Segmenter(undefined, { granularity: 'grapheme' });
                        const segments = Array.from(segmenter.segment(value));
                        const lastGrapheme = segments[segments.length - 1]?.segment;
                        if (lastGrapheme?.trim()) {
                          setIcon(lastGrapheme);
                          setErrors((previous) => ({ ...previous, icon: false }));
                          return;
                        }
                      }
                    } catch (error) {
                      console.error(error);
                    }
                    const characters = Array.from(value);
                    const lastCharacter = characters[characters.length - 1];
                    if (lastCharacter) {
                      setIcon(lastCharacter);
                      setErrors((previous) => ({ ...previous, icon: false }));
                    }
                  }}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' || event.key === 'Tab' || event.metaKey || event.ctrlKey) return;
                    event.preventDefault();
                  }}
                  onClick={async (event) => {
                    event.currentTarget.select();
                    const openedNativePicker = await invoke<boolean>('open_emoji_picker').catch(() => false);
                    if (!openedNativePicker) setIsEmojiMenuOpen((open) => !open);
                  }}
                  onFocus={(event) => event.currentTarget.select()}
                  placeholder="📂"
                  maxLength={64}
                  className={`w-16 theme-input ui-field-radius emoji-input-picker border py-1.5 text-center font-mono text-lg focus:outline-none shadow-inner cursor-pointer select-none transition-colors ${errors.icon ? 'form-field-error' : 'form-field-valid'}`}
                  aria-label={translate('component.binModal.chooseBinIcon')}
                  title={translate('component.binModal.openEmojiPicker')}
                />
              ) : (
                <button
                  ref={emojiTriggerRef}
                  type="button"
                  onClick={() => setIsEmojiMenuOpen((open) => !open)}
                  className={`w-16 theme-input ui-field-radius emoji-input-picker border py-1.5 text-center font-mono text-lg focus:outline-none shadow-inner cursor-pointer select-none transition-colors ${errors.icon ? 'form-field-error' : 'form-field-valid'}`}
                  aria-label={translate('component.binModal.chooseBinIcon')}
                  aria-haspopup="menu"
                  aria-expanded={isEmojiMenuOpen}
                  title={translate('component.binModal.chooseBinIcon')}
                >
                  {formatEmojiIcon(icon)}
                </button>
              )}
              <span className="text-[11px] theme-text-muted">
                {desktopPlatform === 'macos' ? (
                  translate('component.binModal.openEmojiPickerShortcut', { shortcut: translate('component.binModal.commandSpace') })
                ) : translate('component.binModal.chooseAnIconForThisBin')}
              </span>
              {isEmojiMenuOpen && (
                <AnchoredMenu
                  anchor={{ kind: 'element', ref: emojiTriggerRef, align: 'start' }}
                  ariaLabel={translate('component.binModal.chooseBinIcon')}
                  onClose={() => setIsEmojiMenuOpen(false)}
                  className="w-72"
                >
                  <div className="grid grid-cols-8 gap-1" role="group" aria-label={translate('component.binModal.binIcons')}>
                    {BIN_EMOJI_OPTIONS.map(([emoji, labelKey]) => {
                      const label = emojiLabel(labelKey);
                      return (
                      <button
                        key={emoji}
                        type="button"
                        role="menuitemradio"
                        aria-checked={formatEmojiIcon(icon) === emoji}
                        aria-label={label}
                        title={label}
                        className={`theme-menu-item grid h-7 w-7 place-items-center rounded-lg text-base ${formatEmojiIcon(icon) === emoji ? 'is-selected' : ''}`}
                        onClick={() => {
                          setIcon(emoji);
                          setErrors((previous) => ({ ...previous, icon: false }));
                          setIsEmojiMenuOpen(false);
                          emojiTriggerRef.current?.focus();
                        }}
                      >
                        {emoji}
                      </button>
                      );
                    })}
                  </div>
                </AnchoredMenu>
              )}
            </div>
          </div>

          <div className="flex items-center gap-3">
            <span className="w-20 shrink-0 text-end text-xs font-semibold theme-text-muted">{translate('component.binModal.transform')}</span>
            <MenuSelect
              value={transformRef}
              options={[
                { value: '', get label() { return translate('component.binModal.doNothing'); } },
                ...transforms
                  .map((transform, sourceIndex) => {
                    const group = transform.authoringKind === 'manual'
                      ? translate('component.transformationPlayground.manuallyBuiltTransforms')
                      : transform.executionCharacter === 'replayable'
                        ? translate('component.transformationPlayground.plannedLocalTransforms')
                        : translate('component.transformationPlayground.aiAssistedTransforms');
                    const groupOrder = transform.authoringKind === 'manual'
                      ? 2
                      : transform.executionCharacter === 'replayable' ? 1 : 0;
                    return {
                      value: transform.stableRef,
                      label: transform.name,
                      group,
                      groupOrder,
                      sourceIndex,
                    };
                  })
                  .sort((left, right) => left.groupOrder - right.groupOrder || left.sourceIndex - right.sourceIndex)
                  .map(({ groupOrder: _groupOrder, sourceIndex: _sourceIndex, ...option }) => option),
              ]}
              onChange={setTransformRef}
              label={translate('component.binModal.transform')}
              className="min-w-0 flex-1"
              searchable
              searchPlaceholder={translate('component.binModal.searchTransforms')}
            />
          </div>

          {features.protection && (
            <div className="flex items-center gap-3">
              <span className="w-20 shrink-0 text-end text-xs font-semibold theme-text-muted">
                {translate('component.binModal.protect')}
              </span>
              <div
                className={`bin-setting-toggle-well theme-surface flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl border p-3 ${modalTab === 'bin' ? 'cursor-pointer' : 'is-disabled cursor-help'}`}
                title={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotProtectClips') : undefined}
                onClick={(event) => {
                  if (modalTab !== 'bin' || (event.target as HTMLElement).closest('button')) return;
                  setProtectClips((value) => !value);
                }}
              >
                <div className="min-w-0">
                  <div className="text-xs font-semibold theme-text-main">
                    {translate('component.binModal.clipsInThisBinAreSafeFromDeletion')}
                  </div>
                </div>
                <SettingsSwitch
                  checked={modalTab === 'bin' && protectClips}
                  disabled={modalTab === 'smart'}
                  label={translate('component.binModal.clipsInThisBinAreSafeFromDeletion')}
                  ariaLabel={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotProtectClips') : undefined}
                  onClick={() => setProtectClips((value) => !value)}
                />
              </div>
            </div>
          )}

          {features.concealment && (
            <div className="flex items-center gap-3">
              <span className="w-20 shrink-0 text-end text-xs font-semibold theme-text-muted">
                {translate('component.binModal.conceal')}
              </span>
              <div
                className={`bin-setting-toggle-well theme-surface flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl border p-3 ${modalTab === 'bin' ? 'cursor-pointer' : 'is-disabled cursor-help'}`}
                title={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotConcealClips') : undefined}
                onClick={(event) => {
                  if (modalTab !== 'bin' || (event.target as HTMLElement).closest('button')) return;
                  setConcealClips((value) => !value);
                }}
              >
                <div className="min-w-0">
                  <div className="text-xs font-semibold theme-text-main">
                    {translate('component.binModal.clipsInThisBinAreConcealed')}
                  </div>
                </div>
                <SettingsSwitch
                  checked={modalTab === 'bin' && concealClips}
                  disabled={modalTab === 'smart'}
                  label={translate('component.binModal.clipsInThisBinAreConcealed')}
                  ariaLabel={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotConcealClips') : undefined}
                  onClick={() => setConcealClips((value) => !value)}
                />
              </div>
            </div>
          )}

          {/* Smart Bin Multi-Condition Builder */}
          {modalTab === 'smart' && (
            <div className="flex items-start gap-3">
              <span className="w-20 shrink-0 pt-0.5 text-end text-xs font-semibold theme-text-muted">{translate('component.binModal.filter')}</span>
              <div className="min-w-0 flex-1 space-y-2">
                <div className="p-4 theme-surface rounded-2xl border space-y-3">
              {conditions.map((c) => (
                <div key={c.id} className="flex items-center space-x-2">
                  <SmartConditionTargetSelect
                    condition={c}
                    sections={targetSectionsFor(c)}
                    onSelect={(target, value) => handleUpdateCondition(c.id, {
                      target,
                      value,
                      operator: target === 'contains' || target === 'file_extension' || target === 'file_path'
                        ? 'contains'
                        : 'is',
                    })}
                  />

                  {/* Operator Dropdown */}
                  <MenuSelect
                    value={c.operator}
                    onChange={(value) => handleUpdateCondition(c.id, { operator: value as SmartConditionRow['operator'] })}
                    options={[
                      { value: 'is', get label() { return translate('component.binModal.is'); } },
                      { value: 'contains', get label() { return translate('component.pipelineEditorModal.contains'); } },
                    ]}
                    label={translate('component.binModal.conditionOperator')}
                    className="w-24"
                    compact
                  />

                  {/* Dynamic Value Dropdown / Input */}
                  {c.target === 'clip_type' ? (
                    <MenuSelect
                      value={c.value}
                      onChange={(value) => handleUpdateCondition(c.id, { value })}
                      options={[
                        { value: 'text', get label() { return translate('component.analyticsView.text'); } },
                        { value: 'image', get label() { return translate('component.analyticsView.image'); } },
                        { value: 'file', get label() { return translate('component.analyticsView.files'); } },
                      ]}
                      label={translate('component.binModal.clipType')}
                      className="min-w-0 flex-1"
                      compact
                    />
                  ) : c.target === 'file_format' || c.target === 'source' || c.target === 'content_type' ? (
                    <SmartConditionValueInput
                      label={targetLabels[c.target]}
                      value={c.value}
                      choices={targetSectionsFor(c).find((section) => section.target === c.target)?.choices ?? []}
                      onChange={(value) => handleUpdateCondition(c.id, { value })}
                    />
                  ) : c.target === 'origin_kind' ? (
                    <MenuSelect
                      value={c.value}
                      onChange={(value) => handleUpdateCondition(c.id, { value })}
                      options={[
                        { value: 'clipboard_content', get label() { return translate('component.binModal.clipboardContent'); } },
                        { value: 'file_reference', get label() { return translate('component.binModal.fileReference'); } },
                        { value: 'screenshot', get label() { return translate('component.binModal.screenshot'); } },
                        { value: 'command_line', get label() { return translate('component.binModal.commandLine'); } },
                      ]}
                      label={translate('component.binModal.captureMethod')}
                      className="min-w-0 flex-1"
                      compact
                    />
                  ) : c.target === 'file_extension' ? (
                    <input
                      type="text"
                      placeholder={translate('component.binModal.eGPdfZipPng')}
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value.replace(/^\./, '') })}
                      className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                    />
                  ) : c.target === 'file_path' ? (
                    <input
                      type="text"
                      placeholder={translate('component.binModal.eGProjectsOrDownloads')}
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value })}
                      className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                    />
                  ) : (
                    <input
                      type="text"
                      placeholder={translate('component.binModal.eGHttpFunction')}
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value })}
                      className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                    />
                  )}

                  {/* Active + / - Condition Buttons */}
                  <div className="flex items-center space-x-1">
                    <button
                      type="button"
                      onClick={() => handleRemoveCondition(c.id)}
                      disabled={conditions.length <= 1}
                      className={`theme-icon-button p-1.5 rounded border transition-[background-color,border-color,color,transform] ${
                        conditions.length <= 1
                          ? 'opacity-40 cursor-not-allowed'
                          : 'hover:scale-105 active:scale-95'
                      }`}
                      title={translate('component.binModal.removeCondition')}
                    >
                      <Minus className="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      onClick={handleAddCondition}
                      className="theme-icon-button p-1.5 rounded border transition-[background-color,border-color,color,transform] hover:scale-105 active:scale-95"
                      title={translate('component.binModal.addCondition')}
                    >
                      <Plus className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}

              <div className="flex items-center space-x-2 pt-1 theme-text-muted">
                <span>{translate('component.binModal.containClipsThatMatch')}</span>
                <MenuSelect
                  value={matchCondition}
                  onChange={(value) => setMatchCondition(value as 'any' | 'all')}
                  options={[
                    { value: 'any', get label() { return translate('component.binModal.any'); } },
                    { value: 'all', get label() { return translate('component.binModal.all'); } },
                  ]}
                  label={translate('component.binModal.conditionMatching')}
                  className="w-24"
                  compact
                />
                  <span>{translate('component.binModal.conditions')}</span>
                </div>
                </div>
                <p className="text-[10px] theme-text-muted">{translate('component.binModal.chooseWhichClipsAutomaticallyEnterThisSmartBin')}</p>
              </div>
            </div>
          )}

          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose}>{translate('common.cancel')}</AppDialogButton>
            <AppDialogButton type="submit" variant="primary"><SaveButtonContent /></AppDialogButton>
          </AppDialogFooter>
        </form>
      </>}
    </AppDialog>
  );
};
