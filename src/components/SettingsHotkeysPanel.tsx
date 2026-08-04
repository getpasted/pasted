import { useEffect, useRef, useState } from 'react';
import { RotateCcw, ShieldCheck } from 'lucide-react';
import type { AppSettings, Bin, Pipeline } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { HotkeyRecorder } from './HotkeyRecorder';

interface SettingsHotkeysPanelProps {
  settings: AppSettings;
  bins: Bin[];
  pipelines: Pipeline[];
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onRefreshBins?: () => void;
  onRefreshPipelines?: () => void;
}

type AccessibilityStatus = { is_trusted: boolean; is_dev_mode: boolean };
type HotkeySetting = keyof Pick<
  AppSettings,
  | 'seqToggleHotkey'
  | 'seqPopHotkey'
  | 'copyLastPipelineHotkey'
  | 'pasteLastPipelineHotkey'
  | 'openTransformationsHotkey'
  | 'openMainWindowHotkey'
  | 'pasteClip1Hotkey'
  | 'pasteClip2Hotkey'
  | 'pasteClip3Hotkey'
  | 'pasteClip4Hotkey'
  | 'pasteClip5Hotkey'
  | 'pasteClip6Hotkey'
  | 'pasteClip7Hotkey'
  | 'pasteClip8Hotkey'
  | 'pasteClip9Hotkey'
>;

const defaultHotkeys: Partial<AppSettings> = {
  hudHotkey: 'Alt+Shift+V',
  seqToggleHotkey: 'Alt+Shift+C',
  seqPopHotkey: 'Alt+Shift+X',
  copyLastPipelineHotkey: '',
  pasteLastPipelineHotkey: '',
  openTransformationsHotkey: '',
  openMainWindowHotkey: '',
  pasteClip1Hotkey: '',
  pasteClip2Hotkey: '',
  pasteClip3Hotkey: '',
  pasteClip4Hotkey: '',
  pasteClip5Hotkey: '',
  pasteClip6Hotkey: '',
  pasteClip7Hotkey: '',
  pasteClip8Hotkey: '',
  pasteClip9Hotkey: '',
};

const actionHotkeys: Array<{ label: string; key: HotkeySetting; fallback?: string }> = [
  { label: 'Enable/Disable Queue', key: 'seqToggleHotkey', fallback: 'Alt+Shift+C' },
  { label: 'Paste Next Item from Queue', key: 'seqPopHotkey', fallback: 'Alt+Shift+X' },
  { label: 'Copy with Last Pipeline', key: 'copyLastPipelineHotkey' },
  { label: 'Paste with Last Pipeline', key: 'pasteLastPipelineHotkey' },
  { label: 'Open Transformations', key: 'openTransformationsHotkey' },
  { label: 'Toggle Main Window', key: 'openMainWindowHotkey' },
];

function HotkeyRow({ label, value, onChange }: { label: string; value: string | null; onChange: (value: string | null) => void }) {
  return (
    <div className="theme-surface flex items-center justify-between p-2.5 rounded-xl border">
      <span className="font-medium theme-text-main">{label}</span>
      <HotkeyRecorder value={value} onChange={onChange} />
    </div>
  );
}

export function SettingsHotkeysPanel({
  settings,
  bins,
  pipelines,
  onUpdateSettings,
  onRefreshBins,
  onRefreshPipelines,
}: SettingsHotkeysPanelProps) {
  const [accessibilityStatus, setAccessibilityStatus] = useState<AccessibilityStatus | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const permissionRefreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refreshAccessibilityStatus = async () => {
    try {
      setAccessibilityStatus(await invoke<AccessibilityStatus>('check_accessibility_permission'));
    } catch {
      setAccessibilityStatus({ is_trusted: true, is_dev_mode: false });
    }
  };

  useEffect(() => {
    void refreshAccessibilityStatus();
    const interval = window.setInterval(() => void refreshAccessibilityStatus(), 10000);
    window.addEventListener('focus', refreshAccessibilityStatus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', refreshAccessibilityStatus);
      if (permissionRefreshTimer.current) clearTimeout(permissionRefreshTimer.current);
    };
  }, []);

  const updateSettingHotkey = async (key: HotkeySetting, newKey: string | null) => {
    const value = newKey ?? '';
    onUpdateSettings({ [key]: value });
    try {
      await invoke('register_app_setting_hotkey', { key, value });
      setStatusMessage(null);
    } catch (error) {
      console.error(`Failed to register ${key}:`, error);
      setStatusMessage('That shortcut could not be registered. Try a different key combination.');
    }
  };

  const restoreDefaults = async () => {
    onUpdateSettings(defaultHotkeys);
    try {
      await invoke('register_hud_shortcut', { shortcutStr: defaultHotkeys.hudHotkey });
      for (const [key, value] of Object.entries(defaultHotkeys)) {
        if (key !== 'hudHotkey') await invoke('register_app_setting_hotkey', { key, value });
      }
      setStatusMessage('Default shortcuts restored.');
    } catch (error) {
      console.error('Failed to restore default hotkeys:', error);
      setStatusMessage('Some default shortcuts could not be registered.');
    }
  };

  const requestAccessibilityPermission = async () => {
    try {
      await invoke('request_accessibility_permission');
      if (permissionRefreshTimer.current) clearTimeout(permissionRefreshTimer.current);
      permissionRefreshTimer.current = setTimeout(() => void refreshAccessibilityStatus(), 1500);
    } catch (error) {
      console.error('Failed to open Accessibility settings:', error);
      setStatusMessage('Could not open macOS Accessibility settings.');
    }
  };

  return (
    <div className="settings-panel theme-panel p-6 rounded-2xl border space-y-6 text-xs">
      <div className="flex items-center justify-between">
        <h3 className="font-bold text-sm theme-title">Global Application Hotkeys</h3>
        <button type="button" onClick={() => void restoreDefaults()} className="theme-secondary-button flex items-center space-x-1.5 px-3 py-1 border rounded-lg transition-colors">
          <RotateCcw className="w-3.5 h-3.5" />
          <span>Restore Defaults</span>
        </button>
      </div>

      {statusMessage && <p role="status" className="theme-status-info rounded-lg border px-3 py-2 text-[11px]">{statusMessage}</p>}

      <div className="theme-surface p-3.5 rounded-xl border space-y-2">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center space-x-2">
            <ShieldCheck className={`w-4 h-4 ${accessibilityStatus?.is_trusted ? 'theme-status-success-text' : 'theme-status-warning-text'}`} />
            <span className="font-bold text-xs theme-text-main">macOS System Accessibility Permission</span>
          </div>
          <div className="flex items-center space-x-2">
            {accessibilityStatus?.is_dev_mode && <span className="theme-status-info text-[9px] font-mono px-2 py-0.5 rounded border font-bold shrink-0 whitespace-nowrap">DEV MODE</span>}
            <span className={`text-[10px] font-mono font-bold px-2 py-0.5 rounded-full border ${accessibilityStatus?.is_trusted ? 'theme-status-success' : 'theme-status-warning'}`}>
              {accessibilityStatus?.is_trusted ? 'GRANTED' : 'REQUIRED'}
            </span>
            <button type="button" onClick={() => void requestAccessibilityPermission()} className="theme-secondary-button px-2.5 py-1 border rounded-lg text-[10px] font-semibold transition-colors cursor-pointer">Open System Settings</button>
          </div>
        </div>
        <p className="text-[11px] theme-text-muted leading-normal">
          macOS requires Accessibility access for global hotkeys. {accessibilityStatus?.is_dev_mode
            ? <span>Running in <strong>development mode</strong>: grant permission to your active IDE / terminal host application under System Settings &gt; Privacy &amp; Security &gt; Accessibility.</span>
            : <span>Grant permission to <strong>Pasted.app</strong> under System Settings &gt; Privacy &amp; Security &gt; Accessibility.</span>}
        </p>
      </div>

      <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Custom Bin Hotkeys ({bins.length})</h4>
        {bins.length === 0
          ? <p className="theme-subtle-surface text-[11px] theme-text-subtle italic p-2.5 rounded-xl border">No custom bins created yet. Create bins in the sidebar to assign global shortcuts.</p>
          : bins.map((bin) => <HotkeyRow key={bin.id} label={bin.name} value={bin.shortcut ?? null} onChange={async (shortcut) => {
              try {
                await invoke('update_bin_shortcut', { id: bin.id, shortcut });
                onRefreshBins?.();
              } catch (error) {
                console.error('Failed to update bin shortcut:', error);
                setStatusMessage('That bin shortcut could not be registered.');
              }
            }} />)}
      </section>

      <section className="theme-divider space-y-2 pt-3 border-t">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Pipeline Hotkeys ({pipelines.length})</h4>
        <p className="text-[11px] theme-text-muted">Assign shortcuts to run reusable transformations instantly.</p>
        <div className="space-y-2 pt-1 max-h-60 overflow-y-auto pr-1">
          {pipelines.map((pipeline) => <HotkeyRow key={pipeline.id} label={pipeline.name} value={pipeline.shortcut ?? null} onChange={async (shortcut) => {
              try {
                await invoke('update_pipeline_shortcut', { pipelineRef: pipeline.stableRef, shortcut });
                onRefreshPipelines?.();
              } catch (error) {
                console.error('Failed to update Pipeline shortcut:', error);
                setStatusMessage('That Pipeline shortcut could not be registered.');
              }
            }} />)}
        </div>
      </section>

      <section className="theme-divider space-y-2 pt-2 border-t">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Actions</h4>
        <HotkeyRow label="HUD" value={settings.hudHotkey === '' ? null : (settings.hudHotkey || 'Alt+Shift+V')} onChange={async (newKey) => {
          const value = newKey ?? '';
          onUpdateSettings({ hudHotkey: value });
          try {
            await invoke('register_hud_shortcut', { shortcutStr: value });
            setStatusMessage(null);
          } catch (error) {
            console.error('Failed to register HUD shortcut:', error);
            setStatusMessage('That shortcut could not be registered. Try a different key combination.');
          }
        }} />
        {actionHotkeys.map(({ label, key, fallback }) => (
          <HotkeyRow key={key} label={label} value={(settings[key] as string) === '' ? null : ((settings[key] as string) || fallback || null)} onChange={(value) => void updateSettingHotkey(key, value)} />
        ))}
      </section>

      <section className="space-y-2 pt-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Paste Recent Clippings</h4>
        {Array.from({ length: 9 }, (_, index) => index + 1).map((number) => {
          const key = `pasteClip${number}Hotkey` as HotkeySetting;
          return <HotkeyRow key={key} label={`Paste Clipping ${number}`} value={(settings[key] as string) || null} onChange={(value) => void updateSettingHotkey(key, value)} />;
        })}
      </section>
    </div>
  );
}
