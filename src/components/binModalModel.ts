import type { Bin } from '../types';
import { translate, type TranslationKey } from '../localization/runtime';

export interface SmartBinFeatures {
  clipTypes: boolean;
  fileFormats: boolean;
  sources: boolean;
  types: boolean;
  protection: boolean;
  concealment: boolean;
}

export type SmartConditionTarget = 'clip_type' | 'file_format' | 'source' | 'content_type' | 'visual_label' | 'origin_kind' | 'contains' | 'file_extension' | 'file_path';

export interface SmartConditionRow {
  id: string;
  target: SmartConditionTarget;
  operator: 'is' | 'contains';
  value: string;
}

export interface SmartTargetChoice {
  value: string;
  label: string;
  group?: string;
  disabled?: boolean;
}

export interface SmartTargetSection {
  target: SmartConditionTarget;
  label: string;
  choices?: SmartTargetChoice[];
  dividerBefore?: boolean;
}

export const STRUCTURAL_CLIP_TYPES = new Set(['text', 'image', 'file']);

export function normalizeSmartCondition(condition: any, index: number): SmartConditionRow {
  const value = typeof condition?.value === 'string' ? condition.value : '';
  const legacyStructuralType = condition?.type === 'content_type' && STRUCTURAL_CLIP_TYPES.has(value);
  return {
    id: String(index + 1),
    target: legacyStructuralType ? 'clip_type' : condition?.type || 'source',
    operator: condition?.operator || 'is',
    value,
  };
}

export const COLOR_PALETTE = [
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

export const BIN_EMOJI_OPTIONS = [
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

export const emojiLabel = (key: string) => translate(`component.binModal.emoji.${key}` as TranslationKey);

export function defaultSmartCondition(features: SmartBinFeatures): SmartConditionRow {
  if (features.clipTypes) return { id: '1', target: 'clip_type', operator: 'is', value: 'text' };
  if (features.types) return { id: '1', target: 'content_type', operator: 'is', value: 'code' };
  if (features.fileFormats) return { id: '1', target: 'file_format', operator: 'is', value: '' };
  if (features.sources) return { id: '1', target: 'source', operator: 'is', value: '1Password' };
  return { id: '1', target: 'contains', operator: 'contains', value: '' };
}

export function initialBinForm(editingBin: Bin | null | undefined, features: SmartBinFeatures) {
  if (editingBin?.smart_rule) {
    try {
      const parsed = JSON.parse(editingBin.smart_rule);
      return {
        modalTab: 'smart' as const,
        conditions: parsed.conditions?.length > 0
          ? parsed.conditions.map(normalizeSmartCondition)
          : [defaultSmartCondition(features)],
        matchCondition: (parsed.match || 'any') as 'any' | 'all',
      };
    } catch {
      // Fall through to safe manual defaults.
    }
  }
  return {
    modalTab: 'bin' as const,
    conditions: [editingBin ? { ...defaultSmartCondition(features), value: '' } : defaultSmartCondition(features)],
    matchCondition: 'any' as const,
  };
}
