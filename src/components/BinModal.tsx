import React, { useState, useEffect, useRef } from 'react';
import { ChevronDown, Folder, Plus, Minus } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { binsApi } from '../api/bins';
import { Bin, TransformDefinition } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { detectDesktopPlatform } from '../utils/platform';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { AnchoredMenu, MenuDivider, MenuItem, MenuSubmenu } from './AnchoredMenu';
import { MenuSelect } from './MenuSelect';
import { useContentTypes } from './ContentTypeProvider';
import { translate, type TranslationKey } from '../localization/runtime';
import { localizedContentTypeGroupLabel } from '../localization/presentation';
import { contentTypeLabel } from '../utils/contentTypes';
import { SettingsSwitch } from './SettingsSwitch';

interface SmartBinFeatures {
  clipTypes: boolean;
  fileFormats: boolean;
  sources: boolean;
  types: boolean;
  protection: boolean;
}

interface BinModalProps {
  isOpen: boolean;
  editingBin?: Bin | null;
  features: SmartBinFeatures;
  fileFormats: string[];
  sources: string[];
  onClose: () => void;
  onRefreshBins: () => void;
}

type SmartConditionTarget = 'clip_type' | 'file_format' | 'source' | 'content_type' | 'origin_kind' | 'contains' | 'file_extension' | 'file_path';

interface SmartConditionRow {
  id: string;
  target: SmartConditionTarget;
  operator: 'is' | 'contains';
  value: string;
}

interface SmartTargetChoice {
  value: string;
  label: string;
  group?: string;
  disabled?: boolean;
}

interface SmartTargetSection {
  target: SmartConditionTarget;
  label: string;
  choices?: SmartTargetChoice[];
  dividerBefore?: boolean;
}

function SmartConditionTargetSelect({
  condition,
  sections,
  onSelect,
}: {
  condition: SmartConditionRow;
  sections: SmartTargetSection[];
  onSelect: (target: SmartConditionTarget, value: string) => void;
}) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [activeSubmenu, setActiveSubmenu] = useState<SmartConditionTarget | null>(null);
  const selectedSection = sections.find(({ target }) => target === condition.target);

  const close = () => {
    setIsOpen(false);
    setActiveSubmenu(null);
    triggerRef.current?.focus();
  };

  return <>
    <button
      ref={triggerRef}
      type="button"
      className="menu-select-trigger flex w-28 min-w-0 items-center gap-2 rounded-lg border px-2 text-start"
      aria-label={translate('component.binModal.conditionTarget')}
      aria-haspopup="menu"
      aria-expanded={isOpen}
      onClick={() => setIsOpen((open) => !open)}
    >
      <span className="bidi-interface-align min-w-0 flex-1 truncate py-1.5 text-xs font-semibold">
        {selectedSection?.label ?? condition.target}
      </span>
      <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
    </button>
    {isOpen && (
      <AnchoredMenu
        anchor={{ kind: 'element', ref: triggerRef, align: 'start' }}
        ariaLabel={translate('component.binModal.conditionTarget')}
        onClose={close}
        className="w-56"
      >
        {sections.map((section) => <React.Fragment key={section.target}>
          {section.dividerBefore && <MenuDivider />}
          {section.choices ? (
            <MenuSubmenu
              label={section.label}
              open={activeSubmenu === section.target}
              onOpenChange={(open) => setActiveSubmenu((current) => (
                open ? section.target : current === section.target ? null : current
              ))}
              onSelect={() => {
                onSelect(
                  section.target,
                  section.target === 'clip_type'
                    ? condition.target === 'clip_type' ? condition.value : 'text'
                    : condition.target === section.target ? condition.value : '',
                );
                close();
              }}
              panelClassName="w-64 max-h-72 overflow-y-auto"
            >
              {section.choices.map((choice, index) => <React.Fragment key={`${section.target}:${choice.value}`}>
                {choice.group && choice.group !== section.choices?.[index - 1]?.group && (
                  <div className={`theme-text-subtle px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider ${index > 0 ? 'theme-divider mt-1 border-t' : ''}`}>
                    {choice.group}
                  </div>
                )}
                <MenuItem
                  active={condition.target === section.target && condition.value === choice.value}
                  disabled={choice.disabled}
                  role="menuitemradio"
                  aria-checked={condition.target === section.target && condition.value === choice.value}
                  className="px-2.5 py-1.5"
                  onClick={() => {
                    if (choice.disabled) return;
                    onSelect(section.target, choice.value);
                    close();
                  }}
                >
                  {choice.label}
                </MenuItem>
              </React.Fragment>)}
            </MenuSubmenu>
          ) : (
            <MenuItem
              active={condition.target === section.target}
              role="menuitemradio"
              aria-checked={condition.target === section.target}
              className="px-3 py-1.5"
              onClick={() => {
                onSelect(section.target, '');
                close();
              }}
            >
              {section.label}
            </MenuItem>
          )}
        </React.Fragment>)}
      </AnchoredMenu>
    )}
  </>;
}

function SmartConditionValueInput({
  value,
  choices,
  label,
  onChange,
}: {
  value: string;
  choices: SmartTargetChoice[];
  label: string;
  onChange: (value: string) => void;
}) {
  const anchorRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [showAll, setShowAll] = useState(true);
  const selectedChoice = choices.find((choice) => choice.value === value);
  const displayedValue = selectedChoice?.label ?? value;
  const normalizedValue = displayedValue.trim().toLowerCase();
  const visibleChoices = showAll || !normalizedValue
    ? choices
    : choices.filter((choice) => (
      `${choice.label} ${choice.value}`.toLowerCase().includes(normalizedValue)
    ));

  return <div ref={anchorRef} className="theme-input form-field-valid flex min-w-0 flex-1 items-center rounded-lg border">
    <input
      ref={inputRef}
      type="text"
      role="combobox"
      aria-label={label}
      aria-autocomplete="list"
      aria-expanded={isOpen}
      placeholder={label}
      value={displayedValue}
      onFocus={() => {
        setShowAll(true);
        setIsOpen(true);
      }}
      onChange={(event) => {
        setShowAll(false);
        setIsOpen(true);
        onChange(event.target.value);
      }}
      onKeyDown={(event) => {
        if (event.key !== 'ArrowDown') return;
        event.preventDefault();
        setShowAll(true);
        setIsOpen(true);
      }}
      className="bidi-content min-w-0 flex-1 bg-transparent px-3 py-1.5 text-xs font-semibold focus:outline-none"
    />
    <button
      type="button"
      className="theme-text-muted grid self-stretch shrink-0 place-items-center px-2"
      aria-label={label}
      aria-haspopup="menu"
      aria-expanded={isOpen}
      onClick={() => {
        if (isOpen) {
          setIsOpen(false);
          return;
        }
        setShowAll(true);
        setIsOpen(true);
        inputRef.current?.focus({ preventScroll: true });
      }}
    >
      <ChevronDown className={`h-3.5 w-3.5 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
    </button>
    {isOpen && (
      <AnchoredMenu
        anchor={{ kind: 'element', ref: anchorRef, align: 'start', gap: 4 }}
        ariaLabel={label}
        onClose={() => setIsOpen(false)}
        restoreFocus={false}
        className="max-h-72 overflow-y-auto"
        style={{ width: anchorRef.current?.getBoundingClientRect().width ?? 220 }}
      >
        {visibleChoices.map((choice, index) => <React.Fragment key={choice.value}>
          {choice.group && choice.group !== visibleChoices[index - 1]?.group && (
            <div className={`theme-text-subtle px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider ${index > 0 ? 'theme-divider mt-1 border-t' : ''}`}>
              {choice.group}
            </div>
          )}
          <MenuItem
            active={value === choice.value}
            disabled={choice.disabled}
            role="menuitemradio"
            aria-checked={value === choice.value}
            className="px-2.5 py-1.5"
            onClick={() => {
              if (choice.disabled) return;
              onChange(choice.value);
              setIsOpen(false);
            }}
          >
            {choice.label}
          </MenuItem>
        </React.Fragment>)}
        {visibleChoices.length === 0 && (
          <div className="theme-text-subtle px-3 py-4 text-center text-xs">
            {translate('component.menuSelect.noMatches')}
          </div>
        )}
      </AnchoredMenu>
    )}
  </div>;
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
  if (features.types) return { id: '1', target: 'content_type', operator: 'is', value: 'code' };
  if (features.fileFormats) return { id: '1', target: 'file_format', operator: 'is', value: '' };
  if (features.sources) return { id: '1', target: 'source', operator: 'is', value: '1Password' };
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
  fileFormats,
  sources,
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
  const [protectClips, setProtectClips] = useState(() => Boolean(editingBin?.protect_clips));

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
        setProtectClips(Boolean(editingBin.protect_clips));
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
        setProtectClips(false);
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
      : features.types
        ? 'content_type'
        : features.fileFormats
          ? 'file_format'
          : features.sources
            ? 'source'
          : 'contains';
    const defaultVal = target === 'clip_type'
      ? 'text'
      : target === 'file_format'
        ? fileFormats[0] || ''
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
        version: 1,
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
        await binsApi.update(editingBin.id, {
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
        await binsApi.setTransform(editingBin.id, transformRef || null);
        if (modalTab === 'bin' && features.protection) {
          await binsApi.updateProtection(editingBin.id, protectClips);
        }
      } else {
        const created = await binsApi.create({
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
        await binsApi.setTransform(created.id, transformRef || null);
        if (modalTab === 'bin' && features.protection && protectClips) {
          await binsApi.updateProtection(created.id, true);
        }
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
  const isDirty = JSON.stringify({
    modalTab,
    name,
    selectedColor,
    icon,
    conditions,
    matchCondition,
    transformRef,
    protectClips: modalTab === 'bin' && protectClips,
  }) !== JSON.stringify({
    modalTab: initial.modalTab,
    name: editingBin?.name || '',
    selectedColor: editingBin?.color || 'default',
    icon: editingBin ? formatEmojiIcon(editingBin.icon) : '📂',
    conditions: initial.conditions,
    matchCondition: initial.matchCondition,
    transformRef: initialTransformRef.current,
    protectClips: initial.modalTab === 'bin' && Boolean(editingBin?.protect_clips),
  });

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
              <div className="theme-surface flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl border p-3">
                <div className="min-w-0">
                  <div className="text-xs font-semibold theme-text-main">
                    {translate('component.binModal.clipsInThisBinAreSafeFromDeletion')}
                  </div>
                  {modalTab === 'smart' && (
                    <p className="mt-0.5 text-[11px] theme-text-muted">
                      {translate('component.binModal.smartBinsCannotProtectClips')}
                    </p>
                  )}
                </div>
                <SettingsSwitch
                  checked={modalTab === 'bin' && protectClips}
                  disabled={modalTab === 'smart'}
                  label={translate('component.binModal.clipsInThisBinAreSafeFromDeletion')}
                  onClick={() => setProtectClips((value) => !value)}
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
