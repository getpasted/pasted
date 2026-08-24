import type { AppSettings } from './types';
import { translate } from './localization/runtime';
import type { SettingsResetChange } from './components/SettingsResetChanges';
import { settingDefault } from './settingsContract.ts';

export type HotkeySetting = keyof Pick<
  AppSettings,
  | 'seqToggleHotkey' | 'seqPopHotkey' | 'copyLastPipelineHotkey' | 'pasteLastPipelineHotkey'
  | 'openTransformationsHotkey' | 'openMainWindowHotkey' | 'lockAppHotkey'
  | 'pasteClip1Hotkey' | 'pasteClip2Hotkey' | 'pasteClip3Hotkey' | 'pasteClip4Hotkey'
  | 'pasteClip5Hotkey' | 'pasteClip6Hotkey' | 'pasteClip7Hotkey' | 'pasteClip8Hotkey'
  | 'pasteClip9Hotkey'
>;

export const DEFAULT_HOTKEYS: Partial<AppSettings> = {
  hudHotkey: settingDefault('hudHotkey'), seqToggleHotkey: settingDefault('seqToggleHotkey'),
  seqPopHotkey: settingDefault('seqPopHotkey'), copyLastPipelineHotkey: settingDefault('copyLastPipelineHotkey'),
  pasteLastPipelineHotkey: settingDefault('pasteLastPipelineHotkey'), openTransformationsHotkey: settingDefault('openTransformationsHotkey'),
  openMainWindowHotkey: settingDefault('openMainWindowHotkey'), lockAppHotkey: settingDefault('lockAppHotkey'),
  pasteClip1Hotkey: settingDefault('pasteClip1Hotkey'), pasteClip2Hotkey: settingDefault('pasteClip2Hotkey'),
  pasteClip3Hotkey: settingDefault('pasteClip3Hotkey'), pasteClip4Hotkey: settingDefault('pasteClip4Hotkey'),
  pasteClip5Hotkey: settingDefault('pasteClip5Hotkey'), pasteClip6Hotkey: settingDefault('pasteClip6Hotkey'),
  pasteClip7Hotkey: settingDefault('pasteClip7Hotkey'), pasteClip8Hotkey: settingDefault('pasteClip8Hotkey'),
  pasteClip9Hotkey: settingDefault('pasteClip9Hotkey'),
};

export const actionHotkeys: Array<{ label: string; key: HotkeySetting; fallback?: string; feature?: 'queue' | 'transformations' | 'appLock' }> = [
  { get label() { return translate('component.settingsHotkeysPanel.toggleMainWindow'); }, key: 'openMainWindowHotkey' },
  { get label() { return translate('component.settingsHotkeysPanel.lockApp'); }, key: 'lockAppHotkey', fallback: 'Alt+Shift+L', feature: 'appLock' },
  { get label() { return translate('component.settingsHotkeysPanel.enableOrDisableQueue'); }, key: 'seqToggleHotkey', fallback: 'Alt+Shift+C', feature: 'queue' },
  { get label() { return translate('component.settingsHotkeysPanel.pasteNextItemFromQueue'); }, key: 'seqPopHotkey', fallback: 'Alt+Shift+X', feature: 'queue' },
  { get label() { return translate('component.settingsHotkeysPanel.copyWithLastAdvancedTransform'); }, key: 'copyLastPipelineHotkey', feature: 'transformations' },
  { get label() { return translate('component.settingsHotkeysPanel.pasteWithLastAdvancedTransform'); }, key: 'pasteLastPipelineHotkey', feature: 'transformations' },
  { get label() { return translate('component.settingsHotkeysPanel.openTransformations'); }, key: 'openTransformationsHotkey', feature: 'transformations' },
];

function hotkeyLabel(key: string) {
  if (key === 'hudHotkey') return translate('component.settingsHotkeysPanel.hud');
  const action = actionHotkeys.find((candidate) => candidate.key === key);
  if (action) return action.label;
  const number = key.match(/^pasteClip(\d)Hotkey$/)?.[1];
  return translate('component.settingsHotkeysPanel.pasteClipNumber', { number: number ?? '' });
}

export function hotkeyResetChanges(settings: AppSettings): SettingsResetChange[] {
  const unassigned = translate('component.settingsResetChanges.unassigned');
  return Object.entries(DEFAULT_HOTKEYS).flatMap(([key, defaultValue]) => {
    const current = String(settings[key as keyof AppSettings] ?? '');
    const next = String(defaultValue ?? '');
    return current === next ? [] : [{ label: hotkeyLabel(key), before: current || unassigned, after: next || unassigned }];
  });
}
