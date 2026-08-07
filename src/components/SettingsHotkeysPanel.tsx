import { useEffect, useRef, useState } from 'react';
import { Keyboard, MonitorCog, RotateCcw, ShieldCheck, TriangleAlert } from 'lucide-react';
import type { AppSettings, Bin, Pipeline } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { HotkeyRecorder } from './HotkeyRecorder';
import { SettingsPanelHeader } from './SettingsPanelHeader';

interface SettingsHotkeysPanelProps {
  settings: AppSettings;
  bins: Bin[];
  pipelines: Pipeline[];
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onRefreshBins?: () => void;
  onRefreshPipelines?: () => void;
}

type HotkeyCapabilityStatus = {
  platform: 'macos' | 'windows' | 'linux' | 'unsupported';
  backend: 'macos' | 'windows' | 'x11' | 'wayland-portal' | 'unsupported';
  state: 'checking' | 'ready' | 'conflict' | 'unavailable';
  is_trusted: boolean;
  is_dev_mode: boolean;
  configured_count: number;
  registered_count: number;
  issues: Array<{ shortcut: string; description: string; message: string }>;
};
let cachedHotkeyStatus: HotkeyCapabilityStatus | null = null;
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

const actionHotkeys: Array<{ label: string; key: HotkeySetting; fallback?: string; feature?: 'queue' | 'transformations' }> = [
  { label: 'Enable/Disable Queue', key: 'seqToggleHotkey', fallback: 'Alt+Shift+C', feature: 'queue' },
  { label: 'Paste Next Item from Queue', key: 'seqPopHotkey', fallback: 'Alt+Shift+X', feature: 'queue' },
  { label: 'Copy with Last Advanced Transform', key: 'copyLastPipelineHotkey', feature: 'transformations' },
  { label: 'Paste with Last Advanced Transform', key: 'pasteLastPipelineHotkey', feature: 'transformations' },
  { label: 'Open Transformations', key: 'openTransformationsHotkey', feature: 'transformations' },
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
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyCapabilityStatus | null>(cachedHotkeyStatus);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const permissionRefreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refreshHotkeyStatus = async () => {
    try {
      const nextStatus = await invoke<HotkeyCapabilityStatus>('get_hotkey_capability_status');
      cachedHotkeyStatus = nextStatus;
      setHotkeyStatus(nextStatus);
    } catch {
      const fallback: HotkeyCapabilityStatus = {
        platform: 'unsupported', backend: 'unsupported', state: 'unavailable',
        is_trusted: true, is_dev_mode: false, configured_count: 0, registered_count: 0,
        issues: [],
      };
      cachedHotkeyStatus = fallback;
      setHotkeyStatus(fallback);
    }
  };

  useEffect(() => {
    void refreshHotkeyStatus();
    const interval = window.setInterval(() => void refreshHotkeyStatus(), 10000);
    window.addEventListener('focus', refreshHotkeyStatus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', refreshHotkeyStatus);
      if (permissionRefreshTimer.current) clearTimeout(permissionRefreshTimer.current);
    };
  }, []);

  const updateSettingHotkey = async (key: HotkeySetting, newKey: string | null) => {
    const value = newKey ?? '';
    onUpdateSettings({ [key]: value });
    try {
      await invoke('register_app_setting_hotkey', { key, value });
      setStatusMessage(null);
      await refreshHotkeyStatus();
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
      permissionRefreshTimer.current = setTimeout(() => void refreshHotkeyStatus(), 1500);
    } catch (error) {
      console.error('Failed to open Accessibility settings:', error);
      setStatusMessage('Could not open macOS Accessibility settings.');
    }
  };

  const isMac = hotkeyStatus?.platform === 'macos';
  const isBrowserPreview = typeof window !== 'undefined'
    && !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  const hasHotkeyIssues = Boolean(hotkeyStatus && hotkeyStatus.issues.length > 0);
  const capabilityTitle = isMac
    ? 'Accessibility Access'
    : hotkeyStatus?.backend === 'wayland-portal'
      ? 'Wayland System Hotkeys'
      : hotkeyStatus?.backend === 'x11'
        ? 'X11 Global Hotkeys'
        : hotkeyStatus?.platform === 'windows'
          ? 'Windows Global Hotkeys'
          : 'Global Hotkeys';
  const capabilityDescription = isMac
    ? (hotkeyStatus?.is_dev_mode
        ? <>In development, allow your active IDE or terminal under <strong>System Settings › Privacy &amp; Security › Accessibility</strong>.</>
        : <>Allow <strong>Pasted</strong> under <strong>System Settings › Privacy &amp; Security › Accessibility</strong>.</>)
    : hotkeyStatus?.backend === 'wayland-portal'
      ? (hotkeyStatus.state === 'unavailable'
          ? <>This desktop does not provide the XDG Global Shortcuts portal. Pasted cannot register system-wide shortcuts in this Wayland session.</>
          : <>Your desktop securely owns these shortcuts. It may ask you to approve or change them when Pasted registers them.</>)
      : hotkeyStatus?.backend === 'x11'
        ? <>Pasted registers shortcuts directly with X11. Shortcuts already owned by the desktop or another app are reported below.</>
        : hotkeyStatus?.platform === 'windows'
          ? <>Pasted registers shortcuts directly with Windows. Reserved shortcuts and conflicts with other apps are reported below.</>
          : isBrowserPreview
            ? <>This window could not register system-wide shortcuts, so hotkeys may not work correctly.</>
            : <>This platform does not currently provide a supported global-hotkey backend.</>;
  const capabilityBadge = !hotkeyStatus || hotkeyStatus.state === 'checking'
    ? 'CHECKING'
    : isMac && !hotkeyStatus.is_trusted
      ? 'REQUIRED'
      : hotkeyStatus.state === 'unavailable'
        ? 'UNAVAILABLE'
        : hasHotkeyIssues
          ? `${hotkeyStatus.issues.length} CONFLICT${hotkeyStatus.issues.length === 1 ? '' : 'S'}`
          : isMac
            ? 'GRANTED'
            : hotkeyStatus.backend === 'wayland-portal'
              ? 'SYSTEM MANAGED'
              : 'READY';
  const capabilityIsHealthy = Boolean(hotkeyStatus
    && hotkeyStatus.state === 'ready'
    && (!isMac || hotkeyStatus.is_trusted));

  return (
    <div className="space-y-6 text-xs">
      <SettingsPanelHeader
        icon={Keyboard}
        title="Hotkeys"
        description="Shortcuts for Pasted, Bins, and Transforms."
        actions={(
          <button type="button" onClick={() => void restoreDefaults()} className="theme-secondary-button flex items-center space-x-1.5 px-3 py-2 border rounded-lg transition-colors">
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Restore Defaults</span>
          </button>
        )}
      />

      {statusMessage && <p role="status" className="theme-status-info rounded-lg border px-3 py-2 text-[11px]">{statusMessage}</p>}

      <div className="theme-surface p-3.5 rounded-xl border space-y-2.5">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 flex items-center gap-2">
            {hasHotkeyIssues || hotkeyStatus?.state === 'unavailable'
              ? <TriangleAlert className="w-4 h-4 shrink-0 theme-status-warning-text" />
              : isMac
                ? <ShieldCheck className={`w-4 h-4 shrink-0 ${capabilityIsHealthy ? 'theme-status-success-text' : 'theme-status-warning-text'}`} />
                : <MonitorCog className={`w-4 h-4 shrink-0 ${capabilityIsHealthy ? 'theme-status-success-text' : 'theme-text-muted'}`} />}
            <span className="min-w-0 truncate whitespace-nowrap font-bold text-xs theme-text-main">{capabilityTitle}</span>
          </div>
          <div className="shrink-0 flex items-center gap-2">
            {isMac && hotkeyStatus?.is_dev_mode && (
              <span className="theme-status-info whitespace-nowrap text-[9px] font-mono px-2 py-0.5 rounded border font-bold">DEV MODE</span>
            )}
            <span className={`whitespace-nowrap text-center text-[9px] font-mono font-bold px-2 py-0.5 rounded-full border ${capabilityIsHealthy ? 'theme-status-success' : 'theme-status-warning'}`}>
              {capabilityBadge}
            </span>
            {isMac && (
              <button type="button" onClick={() => void requestAccessibilityPermission()} className="theme-secondary-button whitespace-nowrap px-2.5 py-1 border rounded-lg text-[10px] font-semibold transition-colors cursor-pointer">
                Open Settings
              </button>
            )}
          </div>
        </div>
        <p className="text-[11px] theme-text-muted leading-normal">{capabilityDescription}</p>
        {hasHotkeyIssues && (
          <ul className="theme-subtle-surface theme-divide divide-y overflow-hidden rounded-lg border" aria-label="Unavailable hotkeys">
            {hotkeyStatus!.issues.slice(0, 4).map((issue, index) => (
              <li key={`${issue.description}-${issue.shortcut}-${index}`} className="flex items-start justify-between gap-3 px-2.5 py-2 text-[10px]">
                <span className="min-w-0 truncate theme-text-main">{issue.description}</span>
                {issue.shortcut && <kbd className="shrink-0 font-mono theme-text-muted">{issue.shortcut}</kbd>}
              </li>
            ))}
          </ul>
        )}
      </div>

      {settings.enableBins && <section className="space-y-2">
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
      </section>}

      {settings.enableTransformations && pipelines.length > 0 && <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Saved Transform Hotkeys ({pipelines.length})</h4>
        <div className="space-y-2 pt-1 max-h-60 overflow-y-auto pr-1">
          {pipelines.map((pipeline) => <HotkeyRow key={pipeline.id} label={pipeline.name} value={pipeline.shortcut ?? null} onChange={async (shortcut) => {
              try {
                await invoke('update_pipeline_shortcut', { pipelineRef: pipeline.stableRef, shortcut });
                onRefreshPipelines?.();
              } catch (error) {
                console.error('Failed to update Advanced Transform shortcut:', error);
                setStatusMessage('That Advanced Transform shortcut could not be registered.');
              }
            }} />)}
        </div>
      </section>}

      <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Actions</h4>
        {settings.enableHud && <HotkeyRow label="HUD" value={settings.hudHotkey === '' ? null : (settings.hudHotkey || 'Alt+Shift+V')} onChange={async (newKey) => {
          const value = newKey ?? '';
          onUpdateSettings({ hudHotkey: value });
          try {
            await invoke('register_hud_shortcut', { shortcutStr: value });
            setStatusMessage(null);
          } catch (error) {
            console.error('Failed to register HUD shortcut:', error);
            setStatusMessage('That shortcut could not be registered. Try a different key combination.');
          }
        }} />}
        {actionHotkeys.filter(({ feature }) => !feature || settings[feature === 'queue' ? 'enableQueue' : 'enableTransformations']).map(({ label, key, fallback }) => (
          <HotkeyRow key={key} label={label} value={(settings[key] as string) === '' ? null : ((settings[key] as string) || fallback || null)} onChange={(value) => void updateSettingHotkey(key, value)} />
        ))}
      </section>

      <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">Paste Recent Clippings</h4>
        {Array.from({ length: 9 }, (_, index) => index + 1).map((number) => {
          const key = `pasteClip${number}Hotkey` as HotkeySetting;
          return <HotkeyRow key={key} label={`Paste Clipping ${number}`} value={(settings[key] as string) || null} onChange={(value) => void updateSettingHotkey(key, value)} />;
        })}
      </section>
    </div>
  );
}
