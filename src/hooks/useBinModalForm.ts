import { useEffect, useRef, useState } from 'react';

import { binsApi } from '../api/bins';
import type { Bin, TransformDefinition } from '../types';
import type { SmartBinFeatures, SmartConditionRow, SmartConditionTarget } from '../components/binModalModel';
import { STRUCTURAL_CLIP_TYPES, defaultSmartCondition, initialBinForm, normalizeSmartCondition } from '../components/binModalModel';
import { formatEmojiIcon } from '../utils/emoji';
import { safeInvoke as invoke } from '../utils/tauri';

interface UseBinModalFormInput {
  isOpen: boolean;
  editingBin?: Bin | null;
  features: SmartBinFeatures;
  fileFormats: string[];
  contentTypes: Array<{ id: string; isArchived: boolean }>;
  onClose: () => void;
  onRefreshBins: () => void;
}

export function useBinModalForm({
  isOpen,
  editingBin,
  features,
  fileFormats,
  contentTypes,
  onClose,
  onRefreshBins,
}: UseBinModalFormInput) {
  const initial = initialBinForm(editingBin, features);
  const [modalTab, setModalTab] = useState<'bin' | 'smart'>(() => editingBin?.smart_rule ? 'smart' : 'bin');
  const [name, setName] = useState(() => editingBin?.name || '');
  const [selectedColor, setSelectedColor] = useState(() => editingBin?.color || 'default');
  const [icon, setIcon] = useState(() => editingBin ? formatEmojiIcon(editingBin.icon) : '📂');
  const [isEmojiMenuOpen, setIsEmojiMenuOpen] = useState(false);
  const emojiTriggerRef = useRef<HTMLButtonElement>(null);
  const [errors, setErrors] = useState<{ name?: boolean; color?: boolean; icon?: boolean }>({});
  const [installedApps, setInstalledApps] = useState<string[]>([]);
  const [transforms, setTransforms] = useState<TransformDefinition[]>([]);
  const [transformRef, setTransformRef] = useState('');
  const [protectClips, setProtectClips] = useState(() => Boolean(editingBin?.protect_clips));
  const [concealClips, setConcealClips] = useState(() => Boolean(editingBin?.conceal_clips));
  const [conditions, setConditions] = useState<SmartConditionRow[]>(() => initial.conditions);
  const [matchCondition, setMatchCondition] = useState<'any' | 'all'>(() => initial.matchCondition);
  const initialTransformRef = useRef('');

  useEffect(() => {
    if (!isOpen) return;
    setErrors({});
    setIsEmojiMenuOpen(false);
    if (editingBin) {
      setName(editingBin.name);
      setSelectedColor(editingBin.color || 'default');
      setIcon(formatEmojiIcon(editingBin.icon));
      setProtectClips(Boolean(editingBin.protect_clips));
      setConcealClips(Boolean(editingBin.conceal_clips));
      if (editingBin.smart_rule) {
        setModalTab('smart');
        try {
          const parsed = JSON.parse(editingBin.smart_rule);
          setConditions(parsed.conditions?.length > 0
            ? parsed.conditions.map(normalizeSmartCondition)
            : [{ ...defaultSmartCondition(features), value: '' }]);
          setMatchCondition(parsed.match || 'any');
        } catch (error) {
          console.error(error);
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
      setConcealClips(false);
      setModalTab('bin');
      setConditions([defaultSmartCondition(features)]);
      setMatchCondition('any');
    }
    invoke<string[]>('get_installed_applications')
      .then((apps) => setInstalledApps(Array.isArray(apps) ? apps : []))
      .catch(console.error);
    invoke<TransformDefinition[]>('get_transforms')
      .then((items) => setTransforms(Array.isArray(items) ? items : []))
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
  }, [editingBin, isOpen]);

  const addCondition = () => {
    const target: SmartConditionTarget = features.clipTypes ? 'clip_type'
      : features.types ? 'content_type'
      : features.fileFormats ? 'file_format'
      : features.sources ? 'source' : 'contains';
    const value = target === 'clip_type' ? 'text'
      : target === 'file_format' ? fileFormats[0] || ''
      : target === 'source' ? installedApps[0] || 'Safari'
      : target === 'content_type'
        ? contentTypes.find((type) => !type.isArchived && !STRUCTURAL_CLIP_TYPES.has(type.id))?.id || ''
        : '';
    setConditions((current) => [...current, {
      id: String(Date.now() + Math.random()),
      target,
      operator: 'is',
      value,
    }]);
  };

  const removeCondition = (id: string) => {
    if (conditions.length > 1) setConditions((current) => current.filter((condition) => condition.id !== id));
  };

  const updateCondition = (id: string, updates: Partial<SmartConditionRow>) => {
    setConditions((current) => current.map((condition) => condition.id === id ? { ...condition, ...updates } : condition));
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    const nextErrors: { name?: boolean; color?: boolean; icon?: boolean } = {};
    if (!name.trim()) nextErrors.name = true;
    if (!selectedColor) nextErrors.color = true;
    if (!icon.trim()) nextErrors.icon = true;
    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      return;
    }
    setErrors({});
    const smartRule = modalTab === 'smart' ? JSON.stringify({
      version: 1,
      conditions: conditions.map((condition) => ({
        type: condition.target,
        operator: condition.operator,
        value: condition.value.trim(),
      })),
      match: matchCondition,
    }) : null;
    try {
      let binId: number;
      if (editingBin) {
        await binsApi.update(editingBin.id, { name: name.trim(), icon: icon || '📂', color: selectedColor, smartRule });
        binId = editingBin.id;
      } else {
        const created = await binsApi.create({ name: name.trim(), icon: icon || '📂', color: selectedColor, smartRule });
        binId = created.id;
      }
      await binsApi.setTransform(binId, transformRef || null);
      if (modalTab === 'bin' && features.protection && (editingBin || protectClips)) {
        await binsApi.updateProtection(binId, protectClips);
      }
      if (modalTab === 'bin' && features.concealment && (editingBin || concealClips)) {
        await binsApi.updateConcealment(binId, concealClips);
      }
      setName('');
      onRefreshBins();
      onClose();
    } catch (error) {
      console.error(error);
    }
  };

  const isDirty = JSON.stringify({
    modalTab, name, selectedColor, icon, conditions, matchCondition, transformRef,
    protectClips: modalTab === 'bin' && protectClips,
    concealClips: modalTab === 'bin' && concealClips,
  }) !== JSON.stringify({
    modalTab: initial.modalTab,
    name: editingBin?.name || '',
    selectedColor: editingBin?.color || 'default',
    icon: editingBin ? formatEmojiIcon(editingBin.icon) : '📂',
    conditions: initial.conditions,
    matchCondition: initial.matchCondition,
    transformRef: initialTransformRef.current,
    protectClips: initial.modalTab === 'bin' && Boolean(editingBin?.protect_clips),
    concealClips: initial.modalTab === 'bin' && Boolean(editingBin?.conceal_clips),
  });

  return {
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
    addCondition, removeCondition, updateCondition, submit, isDirty,
  };
}

export type BinModalFormController = ReturnType<typeof useBinModalForm>;
