import React, { useState, useEffect, useRef } from 'react';
import { Folder, Plus, Minus } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { Bin, TransformDefinition } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { detectDesktopPlatform } from '../utils/platform';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { AnchoredMenu } from './AnchoredMenu';
import { MenuSelect } from './MenuSelect';
import { useContentTypes } from './ContentTypeProvider';
import { translate, type TranslationKey } from '../localization/runtime';
import { localizedContentTypeGroupLabel } from '../localization/presentation';
import { contentTypeLabel } from '../utils/contentTypes';

interface SmartBinFeatures {
  clipTypes: boolean;
  sources: boolean;
  types: boolean;
}

interface BinModalProps {
  isOpen: boolean;
  editingBin?: Bin | null;
  features: SmartBinFeatures;
  onClose: () => void;
  onRefreshBins: () => void;
}

type SmartConditionTarget = 'clip_type' | 'source' | 'content_type' | 'origin_kind' | 'contains' | 'file_extension' | 'file_path';

interface SmartConditionRow {
  id: string;
  target: SmartConditionTarget;
  operator: 'is' | 'contains';
  value: string;
}

const STRUCTURAL_CLIP_TYPES = new Set(['text', 'image', 'file']);

function normalizeSmartCondition(condition: any, index: number): SmartConditionRow {
  const value = typeof condition?.value === 'string' ? condition.value : '';
  const legacyStructuralType = condition?.type === 'content_type' && STRUCTURAL_CLIP_TYPES.has(value);
  return {
    id: String(index + 1),
    target: legacyStructuralType ? 'clip_type' : condition?.type || 'source',
    operator: condition?.operator || 'is',
    value,
  };
}

const COLOR_PALETTE = [
  { hex: 'default', get label() { return translate('common.default'); } },
  { hex: '#ef4444', get label() { return translate('component.binModal.red'); } },
  { hex: '#f97316', get label() { return translate('component.binModal.orange'); } },
  { hex: '#eab308', get label() { return translate('component.binModal.yellow'); } },
  { hex: '#10b981', get label() { return translate('component.binModal.green'); } },
  { hex: '#ec4899', get label() { return translate('component.binModal.pink'); } },
  { hex: '#8b5cf6', get label() { return translate('component.binModal.purple'); } },
  { hex: '#06b6d4', get label() { return translate('component.binModal.cyan'); } },
  { hex: '#3b82f6', get label() { return translate('component.binModal.blue'); } },
  { hex: '#6b7280', get label() { return translate('component.binModal.gray'); } },
  { hex: '#d97706', get label() { return translate('component.binModal.amber'); } },
];

const BIN_EMOJI_OPTIONS = [
  ['📂', 'folder'], ['📁', 'openFolder'], ['🗂️', 'dividers'], ['🗃️', 'archive'],
  ['📋', 'clipboard'], ['📌', 'pin'], ['🔖', 'bookmark'], ['🏷️', 'label'],
  ['⭐', 'star'], ['✨', 'sparkles'], ['❤️', 'favorite'], ['🔥', 'hot'],
  ['💡', 'idea'], ['🧠', 'knowledge'], ['📝', 'notes'], ['📚', 'reference'],
  ['📄', 'document'], ['📊', 'data'], ['📸', 'screenshot'], ['🖼️', 'image'],
  ['🎨', 'design'], ['🎵', 'audio'], ['🎬', 'video'], ['🎮', 'games'],
  ['🔗', 'links'], ['🌐', 'web'], ['💬', 'messages'], ['📧', 'email'],
  ['💻', 'computer'], ['⌨️', 'code'], ['🧰', 'tools'], ['⚙️', 'settings'],
  ['🔐', 'secure'], ['🛡️', 'protected'], ['🔑', 'keys'], ['🧪', 'testing'],
  ['✅', 'complete'], ['🚧', 'inProgress'], ['⚠️', 'important'], ['🗑️', 'trash'],
  ['🏠', 'home'], ['💼', 'work'], ['👤', 'personal'], ['👥', 'people'],
  ['🛒', 'shopping'], ['💰', 'finance'], ['✈️', 'travel'], ['📍', 'places'],
  ['🍔', 'food'], ['☕', 'coffee'], ['🏋️', 'fitness'], ['💊', 'health'],
  ['🌱', 'growth'], ['🌙', 'later'], ['🚀', 'launch'], ['🎯', 'goals'],
] as const;

const emojiLabel = (key: string) => translate(`component.binModal.emoji.${key}` as TranslationKey);

function defaultSmartCondition(features: SmartBinFeatures): SmartConditionRow {
  if (features.clipTypes) return { id: '1', target: 'clip_type', operator: 'is', value: 'text' };
  if (features.sources) return { id: '1', target: 'source', operator: 'is', value: '1Password' };
  if (features.types) return { id: '1', target: 'content_type', operator: 'is', value: 'code' };
  return { id: '1', target: 'contains', operator: 'contains', value: '' };
}

function initialBinForm(editingBin: Bin | null | undefined, features: SmartBinFeatures) {
  if (editingBin?.smart_rule) {
    try {
      const parsed = JSON.parse(editingBin.smart_rule);
      return {
        modalTab: 'smart',
        conditions: parsed.conditions?.length > 0
          ? parsed.conditions.map(normalizeSmartCondition)
          : [defaultSmartCondition(features)],
        matchCondition: parsed.match || 'any',
      };
    } catch {
      // Fall through to the safe manual defaults used by the editor.
    }
  }
  return {
    modalTab: 'bin',
    conditions: [editingBin
      ? { ...defaultSmartCondition(features), value: '' }
      : defaultSmartCondition(features)],
    matchCondition: 'any',
  };
}

export const BinModal: React.FC<BinModalProps> = ({
  isOpen,
  editingBin,
  features,
  onClose,
  onRefreshBins,
}) => {
  const { definitions: contentTypes, groups: contentTypeGroups } = useContentTypes();
  const [modalTab, setModalTab] = useState<'bin' | 'smart'>(() => {
    if (editingBin?.smart_rule) return 'smart';
    return 'bin';
  });
  const [name, setName] = useState(() => editingBin?.name || '');
  const [selectedColor, setSelectedColor] = useState(() => editingBin?.color || 'default');
  const [icon, setIcon] = useState(() => (editingBin ? formatEmojiIcon(editingBin.icon) : '📂'));
  const [isEmojiMenuOpen, setIsEmojiMenuOpen] = useState(false);
  const emojiTriggerRef = useRef<HTMLButtonElement>(null);
  const desktopPlatform = detectDesktopPlatform();

  // Form Validation State
  const [errors, setErrors] = useState<{ name?: boolean; color?: boolean; icon?: boolean }>({});

  // Installed OS Apps state
  const [installedApps, setInstalledApps] = useState<string[]>([]);
  const [transforms, setTransforms] = useState<TransformDefinition[]>([]);
  const [transformRef, setTransformRef] = useState('');

  // Multi-condition Smart Rules state
  const [conditions, setConditions] = useState<SmartConditionRow[]>(() => {
    if (editingBin?.smart_rule) {
      try {
        const parsed = JSON.parse(editingBin.smart_rule);
        if (parsed.conditions && parsed.conditions.length > 0) {
          return parsed.conditions.map(normalizeSmartCondition);
        }
      } catch (e) {
        console.error(e);
      }
    }
    return [defaultSmartCondition(features)];
  });
  const [matchCondition, setMatchCondition] = useState<'any' | 'all'>(() => {
    if (editingBin?.smart_rule) {
      try {
        const parsed = JSON.parse(editingBin.smart_rule);
        return parsed.match || 'any';
      } catch (e) {
        console.error(e);
      }
    }
    return 'any';
  });

  const initialTransformRef = React.useRef('');

  useEffect(() => {
    if (isOpen) {
      setErrors({});
      setIsEmojiMenuOpen(false);

      if (editingBin) {
        setName(editingBin.name);
        setSelectedColor(editingBin.color || 'default');
        setIcon(formatEmojiIcon(editingBin.icon));
        if (editingBin.smart_rule) {
          setModalTab('smart');
          try {
            const parsed = JSON.parse(editingBin.smart_rule);
            if (parsed.conditions && parsed.conditions.length > 0) {
              setConditions(
                parsed.conditions.map(normalizeSmartCondition)
              );
            } else {
              setConditions([{ ...defaultSmartCondition(features), value: '' }]);
            }
            setMatchCondition(parsed.match || 'any');
          } catch (e) {
            console.error(e);
            setConditions([{ ...defaultSmartCondition(features), value: '' }]);
          }
        } else {
          setModalTab('bin');
          setConditions([{ ...defaultSmartCondition(features), value: '' }]);
        }
      } else {
        setName('');
        setSelectedColor('default');
        setIcon('📂');
        setModalTab('bin');
        setConditions([defaultSmartCondition(features)]);
      }

      invoke<string[]>('get_installed_applications')
        .then((apps) => {
          setInstalledApps(Array.isArray(apps) ? apps : []);
        })
        .catch(console.error);
      invoke<TransformDefinition[]>('get_transforms')
        .then((savedTransforms) => setTransforms(Array.isArray(savedTransforms) ? savedTransforms : []))
        .catch(console.error);
      if (editingBin) {
        invoke<string | null>('get_bin_transform_ref', { binId: editingBin.id })
          .then((value) => {
            initialTransformRef.current = value || '';
            setTransformRef(value || '');
          })
          .catch(console.error);
      } else {
        initialTransformRef.current = '';
        setTransformRef('');
      }
    }
  }, [isOpen, editingBin]);

  if (!isOpen) return null;

  const handleAddCondition = () => {
    const target: SmartConditionTarget = features.clipTypes
      ? 'clip_type'
      : features.sources
        ? 'source'
        : features.types
          ? 'content_type'
          : 'contains';
    const defaultVal = target === 'clip_type'
      ? 'text'
      : target === 'source'
        ? installedApps[0] || 'Safari'
        : target === 'content_type'
          ? contentTypes.find((type) => !type.isArchived && !STRUCTURAL_CLIP_TYPES.has(type.id))?.id || ''
          : '';
    setConditions((prev) => [
      ...prev,
      {
        id: String(Date.now() + Math.random()),
        target,
        operator: 'is',
        value: defaultVal,
      },
    ]);
  };

  const handleRemoveCondition = (id: string) => {
    if (conditions.length <= 1) return;
    setConditions((prev) => prev.filter((c) => c.id !== id));
  };

  const handleUpdateCondition = (id: string, updates: Partial<SmartConditionRow>) => {
    setConditions((prev) =>
      prev.map((c) => (c.id === id ? { ...c, ...updates } : c))
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validation Check
    const newErrors: { name?: boolean; color?: boolean; icon?: boolean } = {};
    if (!name.trim()) newErrors.name = true;
    if (!selectedColor) newErrors.color = true;
    if (!icon || !icon.trim()) newErrors.icon = true;

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }

    setErrors({});

    let smartRuleJson: string | null = null;
    if (modalTab === 'smart') {
      smartRuleJson = JSON.stringify({
        conditions: conditions.map((c) => ({
          type: c.target,
          operator: c.operator,
          value: c.value.trim(),
        })),
        match: matchCondition,
      });
    }

    try {
      if (editingBin) {
        await invoke('update_bin', {
          id: editingBin.id,
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
        await invoke('set_bin_transform_ref', { binId: editingBin.id, transformRef: transformRef || null });
      } else {
        const created = await invoke<Bin>('create_bin', {
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
        await invoke('set_bin_transform_ref', { binId: created.id, transformRef: transformRef || null });
      }
      setName('');
      onRefreshBins();
      onClose();
    } catch (err) {
      console.error(err);
    }
  };

  const initial = initialBinForm(editingBin, features);
  const activeContentTypes = contentTypes.filter((type) => (
    !type.isArchived && !STRUCTURAL_CLIP_TYPES.has(type.id)
  ));
  const targetLabels: Record<SmartConditionTarget, string> = {
    clip_type: translate('component.binModal.clipType'),
    source: translate('component.binModal.source'),
    content_type: translate('component.binModal.contentType2'),
    origin_kind: translate('component.binModal.captureMethod'),
    contains: translate('component.binModal.textContent'),
    file_extension: translate('component.binModal.fileExtension'),
    file_path: translate('component.binModal.filePath'),
  };
  const targetOptions = [
    ...(features.clipTypes ? [{ value: 'clip_type', label: targetLabels.clip_type }] : []),
    ...(features.types ? [{ value: 'content_type', label: targetLabels.content_type }] : []),
    ...(features.sources ? [{ value: 'source', label: targetLabels.source }] : []),
    { value: 'origin_kind', label: targetLabels.origin_kind },
    { value: 'contains', label: targetLabels.contains },
    { value: 'file_extension', label: targetLabels.file_extension, dividerBefore: true },
    { value: 'file_path', label: targetLabels.file_path },
  ];
  const isDirty = JSON.stringify({
    modalTab,
    name,
    selectedColor,
    icon,
    conditions,
    matchCondition,
    transformRef,
  }) !== JSON.stringify({
    modalTab: initial.modalTab,
    name: editingBin?.name || '',
    selectedColor: editingBin?.color || 'default',
    icon: editingBin ? formatEmojiIcon(editingBin.icon) : '📂',
    conditions: initial.conditions,
    matchCondition: initial.matchCondition,
    transformRef: initialTransformRef.current,
  });

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="bin-modal-title"
      isDirty={isDirty}
      panelClassName="bin-modal-card theme-panel w-full max-w-xl max-h-[90vh] border shadow-2xl overflow-hidden flex flex-col font-sans"
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

          {/* Smart Bin Multi-Condition Builder */}
          {modalTab === 'smart' && (
            <div className="flex items-start gap-3">
              <span className="w-20 shrink-0 pt-0.5 text-end text-xs font-semibold theme-text-muted">{translate('component.binModal.filter')}</span>
              <div className="min-w-0 flex-1 space-y-2">
                <div className="p-4 theme-surface rounded-2xl border space-y-3">
              {conditions.map((c) => (
                <div key={c.id} className="flex items-center space-x-2">
                  {/* Condition Target Dropdown */}
                  <MenuSelect
                    value={c.target}
                    onChange={(value) => {
                      const newTarget = value as SmartConditionRow['target'];
                      const newDefaultVal =
                        newTarget === 'clip_type'
                          ? 'text'
                          : newTarget === 'source'
                          ? installedApps[0] || 'Safari'
                          : newTarget === 'content_type'
                          ? activeContentTypes[0]?.id || ''
                          : newTarget === 'origin_kind'
                          ? 'clipboard_content'
                          : '';
                      handleUpdateCondition(c.id, { target: newTarget, value: newDefaultVal });
                    }}
                    options={targetOptions.some(({ value }) => value === c.target)
                      ? targetOptions
                      : [{ value: c.target, label: targetLabels[c.target], disabled: true }, ...targetOptions]}
                    label={translate('component.binModal.conditionTarget')}
                    className="w-28"
                    compact
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
                  ) : c.target === 'source' ? (
                    <MenuSelect
                      value={c.value}
                      onChange={(value) => handleUpdateCondition(c.id, { value })}
                      options={installedApps.length > 0
                        ? [
                          ...(!installedApps.includes(c.value) && c.value ? [{ value: c.value, label: c.value }] : []),
                          ...installedApps.map((appName) => ({ value: appName, label: appName })),
                        ]
                        : [{ value: c.value, label: c.value || translate('component.binModal.noDetectedApps'), disabled: true }]}
                      label={translate('component.binModal.sourceApp')}
                      className="min-w-0 flex-1"
                      compact
                    />
                  ) : c.target === 'content_type' ? (
                    <MenuSelect
                      value={c.value}
                      onChange={(value) => handleUpdateCondition(c.id, { value })}
                      options={[
                        ...(!activeContentTypes.some(({ id }) => id === c.value) && c.value
                          ? [{ value: c.value, label: contentTypeLabel(c.value), disabled: true }]
                          : []),
                        ...activeContentTypes.map((type) => ({
                        value: type.id,
                        label: contentTypeLabel(type.id),
                        group: (() => {
                          const group = contentTypeGroups.find(({ id }) => id === type.group);
                          return group ? localizedContentTypeGroupLabel(group.id, group.label, group.isBuiltin, group.defaults?.label) : type.group;
                        })(),
                      }))]}
                      label={translate('component.binModal.contentType')}
                      className="min-w-0 flex-1"
                      compact
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
