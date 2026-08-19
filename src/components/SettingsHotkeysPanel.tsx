import { useEffect, useRef, useState } from 'react';
import { Keyboard, MonitorCog, RotateCcw, ShieldCheck, TriangleAlert } from 'lucide-react';
import type { AppSettings, Bin, ManualTransform } from '../types';
import { transformsApi } from '../api/transforms';
import { binsApi } from '../api/bins';
import { safeInvoke as invoke } from '../utils/tauri';
import { HotkeyRecorder } from './HotkeyRecorder';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { OverflowText } from './OverflowText';
import { useToast } from './ToastProvider';
import { ActionButton } from './AppDialogLayout';
import { listen } from '@tauri-apps/api/event';
import { translate } from '../localization/runtime';

interface SettingsHotkeysPanelProps {
  settings: AppSettings;
  bins: Bin[];
  manualTransforms: ManualTransform[];
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onRefreshBins?: () => void;
  onRefreshManualTransforms?: () => void;
}

type HotkeyCapabilityStatus = {
  platform: 'macos' | 'windows' | 'linux' | 'unsupported';
  backend: 'macos' | 'windows' | 'x11' | 'wayland-portal' | 'unsupported';
  state: 'checking' | 'ready' | 'conflict' | 'unavailable' | 'disabled';
  is_trusted: boolean;
  is_dev_mode: boolean;
  configured_count: number;
  registered_count: number;
  issues: Array<{ hotkey: string; description: string; message: string }>;
  bindings: Array<{ id: string; description: string; trigger: string }>;
};
type ClipHotkeyAssignment = { clipId: number; hotkey: string };
let cachedHotkeyStatus: HotkeyCapabilityStatus | null = null;
type HotkeySetting = keyof Pick<
  AppSettings,
  | 'seqToggleHotkey'
  | 'seqPopHotkey'
  | 'copyLastPipelineHotkey'
  | 'pasteLastPipelineHotkey'
  | 'openTransformationsHotkey'
  | 'openMainWindowHotkey'
  | 'lockAppHotkey'
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
  lockAppHotkey: 'Alt+Shift+L',
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

const actionHotkeys: Array<{ label: string; key: HotkeySetting; fallback?: string; feature?: 'queue' | 'transformations' | 'appLock' }> = [
  { get label() { return translate('component.settingsHotkeysPanel.toggleMainWindow'); }, key: 'openMainWindowHotkey' },
  { get label() { return translate('component.settingsHotkeysPanel.lockApp'); }, key: 'lockAppHotkey', fallback: 'Alt+Shift+L', feature: 'appLock' },
  { get label() { return translate('component.settingsHotkeysPanel.enableOrDisableQueue'); }, key: 'seqToggleHotkey', fallback: 'Alt+Shift+C', feature: 'queue' },
  { get label() { return translate('component.settingsHotkeysPanel.pasteNextItemFromQueue'); }, key: 'seqPopHotkey', fallback: 'Alt+Shift+X', feature: 'queue' },
  { get label() { return translate('component.settingsHotkeysPanel.copyWithLastAdvancedTransform'); }, key: 'copyLastPipelineHotkey', feature: 'transformations' },
  { get label() { return translate('component.settingsHotkeysPanel.pasteWithLastAdvancedTransform'); }, key: 'pasteLastPipelineHotkey', feature: 'transformations' },
  { get label() { return translate('component.settingsHotkeysPanel.openTransformations'); }, key: 'openTransformationsHotkey', feature: 'transformations' },
];

function HotkeyRow({ label, value, onChange }: { label: string; value: string | null; onChange: (value: string | null) => void }) {
  return (
    <div className="theme-divider flex items-center justify-between gap-3 border-b p-2.5 last:border-b-0">
      <span className="font-medium theme-text-main">{label}</span>
      <HotkeyRecorder value={value} onChange={onChange} />
    </div>
  );
}

export function SettingsHotkeysPanel({
  settings,
  bins,
  manualTransforms,
  onUpdateSettings,
  onRefreshBins,
  onRefreshManualTransforms,
}: SettingsHotkeysPanelProps) {
  const { showToast } = useToast();
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyCapabilityStatus | null>(cachedHotkeyStatus);
  const [clipHotkeys, setClipHotkeys] = useState<ClipHotkeyAssignment[]>([]);
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
        issues: [], bindings: [],
      };
      cachedHotkeyStatus = fallback;
      setHotkeyStatus(fallback);
    }
  };

  const refreshClipHotkeys = async () => {
    try {
      setClipHotkeys(await invoke<ClipHotkeyAssignment[]>('get_clip_hotkey_assignments'));
    } catch (error) {
      console.error('Failed to load clip hotkeys:', error);
    }
  };

  useEffect(() => {
    void refreshHotkeyStatus();
    void refreshClipHotkeys();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen('hotkey-registration-changed', () => {
      void refreshHotkeyStatus();
      void refreshClipHotkeys();
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    }).catch(console.error);
    window.addEventListener('focus', refreshHotkeyStatus);
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener('focus', refreshHotkeyStatus);
      if (permissionRefreshTimer.current) clearTimeout(permissionRefreshTimer.current);
    };
  }, []);

  const updateSettingHotkey = async (key: HotkeySetting, newKey: string | null) => {
    const value = newKey ?? '';
    const previousValue = settings[key] ?? '';
    onUpdateSettings({ [key]: value });
    try {
      await invoke('register_app_setting_hotkey', { key, value });
      await refreshHotkeyStatus();
    } catch (error) {
      onUpdateSettings({ [key]: previousValue });
      console.error(`Failed to register ${key}:`, error);
      showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.thatHotkeyCouldNotBeRegisteredTryADifferentKeyCombination'); } });
    }
  };

  const restoreDefaults = async () => {
    const previousValues = Object.fromEntries(
      Object.keys(defaultHotkeys).map((key) => [key, settings[key as keyof AppSettings] ?? '']),
    ) as Partial<AppSettings>;
    onUpdateSettings(defaultHotkeys);
    try {
      await invoke('register_app_setting_hotkeys', { values: defaultHotkeys });
      await refreshHotkeyStatus();
      showToast({ tone: 'success', get message() { return translate('component.settingsHotkeysPanel.defaultHotkeysRestored'); } });
    } catch (error) {
      onUpdateSettings(previousValues);
      console.error('Failed to restore default hotkeys:', error);
      showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.someDefaultHotkeysCouldNotBeRegistered'); } });
    }
  };

  const requestAccessibilityPermission = async () => {
    try {
      await invoke('request_accessibility_permission');
      if (permissionRefreshTimer.current) clearTimeout(permissionRefreshTimer.current);
      permissionRefreshTimer.current = setTimeout(() => void refreshHotkeyStatus(), 1500);
    } catch (error) {
      console.error('Failed to open Accessibility settings:', error);
      showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.couldNotOpenMacosAccessibilitySettings'); } });
    }
  };

  const isMac = hotkeyStatus?.platform === 'macos';
  const isBrowserPreview = typeof window !== 'undefined'
    && !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  const hasHotkeyIssues = Boolean(hotkeyStatus && hotkeyStatus.issues.length > 0);
  const capabilityTitle = isMac
    ? translate('component.settingsHotkeysPanel.accessibilityAccess')
    : hotkeyStatus?.backend === 'wayland-portal'
      ? translate('component.settingsHotkeysPanel.waylandSystemHotkeys')
      : hotkeyStatus?.backend === 'x11'
        ? translate('component.settingsHotkeysPanel.x11GlobalHotkeys')
        : hotkeyStatus?.platform === 'windows'
          ? translate('component.settingsHotkeysPanel.windowsGlobalHotkeys')
          : translate('component.settingsHotkeysPanel.globalHotkeys');
  const capabilityDescription = isMac
    ? (hotkeyStatus?.is_dev_mode
        ? translate('component.settingsHotkeysPanel.developmentAccessibilityInstructions', { settingsPath: translate('component.settingsHotkeysPanel.systemSettingsPrivacySecurityAccessibility') })
        : translate('component.settingsHotkeysPanel.accessibilityInstructions', { app: translate('component.settingsHotkeysPanel.pasted'), settingsPath: translate('component.settingsHotkeysPanel.systemSettingsPrivacySecurityAccessibility') }))
    : hotkeyStatus?.backend === 'wayland-portal'
      ? (hotkeyStatus.state === 'unavailable'
          ? <>{translate('component.settingsHotkeysPanel.thisDesktopDoesNotProvideTheXdgGlobalShortcutsPortalSoSystem')}</>
          : <>{translate('component.settingsHotkeysPanel.theDesktopSecurelyOwnsTheseHotkeysAndMayRequestApprovalOrChanges')}</>)
      : hotkeyStatus?.backend === 'x11'
        ? <>{translate('component.settingsHotkeysPanel.hotkeysRegisterDirectlyWithX11ConflictsWithTheDesktopOrAnotherApp')}</>
        : hotkeyStatus?.platform === 'windows'
          ? <>{translate('component.settingsHotkeysPanel.hotkeysRegisterDirectlyWithWindowsReservedHotkeysAndConflictsWithOtherApps')}</>
          : isBrowserPreview
            ? <>{translate('component.settingsHotkeysPanel.thisWindowCouldNotRegisterSystemWideHotkeysSoHotkeysMayNot')}</>
            : <>{translate('component.settingsHotkeysPanel.thisPlatformDoesNotCurrentlyProvideASupportedGlobalHotkeyBackend')}</>;
  const capabilityBadge = !hotkeyStatus || hotkeyStatus.state === 'checking'
    ? translate('component.settingsHotkeysPanel.checking')
    : isMac && !hotkeyStatus.is_trusted
      ? translate('component.settingsHotkeysPanel.required')
      : hotkeyStatus.state === 'unavailable'
        ? translate('component.settingsHotkeysPanel.unavailable')
        : hasHotkeyIssues
          ? translate('component.settingsHotkeysPanel.conflicts', { count: hotkeyStatus.issues.length })
          : isMac
            ? translate('component.settingsHotkeysPanel.granted')
            : hotkeyStatus.backend === 'wayland-portal'
              ? translate('component.settingsHotkeysPanel.systemManaged')
              : translate('component.settingsHotkeysPanel.ready');
  const capabilityIsHealthy = Boolean(hotkeyStatus
    && hotkeyStatus.state === 'ready'
    && (!isMac || hotkeyStatus.is_trusted));

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Keyboard}
        title={translate('component.settingsHotkeysPanel.hotkeys')}
        description={translate('component.settingsHotkeysPanel.globalHotkeysBinActionsAndTransformTriggers')}
        actions={(
          <ActionButton onClick={() => void restoreDefaults()}>
            <RotateCcw className="w-3.5 h-3.5" />
            <span>{translate('common.reset')}</span>
          </ActionButton>
        )}
      />

      <div className="theme-surface p-3.5 rounded-xl border space-y-2.5">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 flex items-center gap-2">
            {hasHotkeyIssues || hotkeyStatus?.state === 'unavailable'
              ? <TriangleAlert className="w-4 h-4 shrink-0 theme-status-warning-text" />
              : isMac
                ? <ShieldCheck className={`w-4 h-4 shrink-0 ${capabilityIsHealthy ? 'theme-status-success-text' : 'theme-status-warning-text'}`} />
                : <MonitorCog className={`w-4 h-4 shrink-0 ${capabilityIsHealthy ? 'theme-status-success-text' : 'theme-text-muted'}`} />}
            <OverflowText text={capabilityTitle} className="min-w-0 truncate whitespace-nowrap font-bold text-xs theme-text-main" />
          </div>
          <div className="shrink-0 flex items-center gap-2">
            {isMac && hotkeyStatus?.is_dev_mode && (
              <span className="theme-status-info whitespace-nowrap text-[9px] font-mono px-2 py-0.5 rounded border font-bold">{translate('component.settingsHotkeysPanel.devMode')}</span>
            )}
            <span className={`whitespace-nowrap text-center text-[9px] font-mono font-bold px-2 py-0.5 rounded-full border ${capabilityIsHealthy ? 'theme-status-success' : 'theme-status-warning'}`}>
              {capabilityBadge}
            </span>
            {isMac && (
              <ActionButton onClick={() => void requestAccessibilityPermission()} className="min-h-7 whitespace-nowrap px-2.5 text-[10px]">
                {translate('component.settingsHotkeysPanel.openSettings')}
              </ActionButton>
            )}
          </div>
        </div>
        <p className="text-[11px] theme-text-muted leading-normal">{capabilityDescription}</p>
        {hasHotkeyIssues && (
          <ul className="theme-subtle-surface theme-divide divide-y overflow-hidden rounded-lg border" aria-label={translate('component.settingsHotkeysPanel.unavailableHotkeys')}>
            {hotkeyStatus!.issues.slice(0, 4).map((issue, index) => (
              <li key={`${issue.description}-${issue.hotkey}-${index}`} className="flex items-start justify-between gap-3 px-2.5 py-2 text-[10px]">
                <OverflowText text={issue.description} className="min-w-0 truncate theme-text-main" />
                {issue.hotkey && <kbd className="shrink-0 font-mono theme-text-muted">{issue.hotkey}</kbd>}
              </li>
            ))}
          </ul>
        )}
        {hotkeyStatus?.backend === 'wayland-portal' && (hotkeyStatus.bindings?.length ?? 0) > 0 && (
          <ul className="theme-subtle-surface theme-divide divide-y overflow-hidden rounded-lg border" aria-label={translate('component.settingsHotkeysPanel.systemManagedHotkeys')}>
            {(hotkeyStatus.bindings ?? []).map((binding) => (
              <li key={binding.id} className="flex items-start justify-between gap-3 px-2.5 py-2 text-[10px]">
                <OverflowText text={binding.description} className="min-w-0 truncate theme-text-main" />
                <kbd className="shrink-0 font-mono theme-text-muted">{binding.trigger}</kbd>
              </li>
            ))}
          </ul>
        )}
      </div>

      <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">{translate('component.settingsHotkeysPanel.actions')}</h4>
        <div className="theme-surface overflow-hidden rounded-xl border">
          {settings.enableHud && <HotkeyRow label={translate('component.settingsHotkeysPanel.hud')} value={settings.hudHotkey === '' ? null : (settings.hudHotkey || translate('component.settingsHotkeysPanel.altShiftV'))} onChange={async (newKey) => {
            const value = newKey ?? '';
            const previousValue = settings.hudHotkey ?? 'Alt+Shift+V';
            onUpdateSettings({ hudHotkey: value });
            try {
              await invoke('register_hud_hotkey', { hotkey: value });
            } catch (error) {
              onUpdateSettings({ hudHotkey: previousValue });
              console.error('Failed to register HUD hotkey:', error);
              showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.thatHotkeyCouldNotBeRegisteredTryADifferentKeyCombination'); } });
            }
          }} />}
          {actionHotkeys.filter(({ feature }) => !feature || settings[feature === 'queue' ? 'enableQueue' : feature === 'transformations' ? 'enableTransformations' : 'enableAppLock']).map(({ label, key, fallback }) => (
            <HotkeyRow key={key} label={label} value={(settings[key] as string) === '' ? null : ((settings[key] as string) || fallback || null)} onChange={(value) => void updateSettingHotkey(key, value)} />
          ))}
        </div>
      </section>

      {settings.enableBins && <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">{translate('component.settingsHotkeysPanel.customBinHotkeys')}{bins.length})</h4>
        <div className="theme-surface overflow-hidden rounded-xl border">
          {bins.length === 0
            ? <p className="theme-text-subtle p-2.5 text-[11px] italic">{translate('component.settingsHotkeysPanel.noCustomBinsCreatedYetCreateBinsInTheSidebarToAssign')}</p>
            : bins.map((bin) => <HotkeyRow key={bin.id} label={bin.name} value={bin.hotkey ?? null} onChange={async (hotkey) => {
              try {
                await binsApi.updateHotkey(bin.id, hotkey);
                onRefreshBins?.();
              } catch (error) {
                console.error('Failed to update Bin hotkey:', error);
                showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.thatBinHotkeyCouldNotBeRegistered'); } });
              }
            }} />)}
        </div>
      </section>}

      {settings.enableProtection && <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">{translate('component.settingsHotkeysPanel.clipHotkeysCount', { count: clipHotkeys.length })}</h4>
        <div className="theme-surface overlay-scroll-region max-h-60 overflow-y-auto rounded-xl border">
          {clipHotkeys.length === 0
            ? <p className="theme-text-subtle p-2.5 text-[11px] italic">{translate('component.settingsHotkeysPanel.noClipHotkeys')}</p>
            : clipHotkeys.map(({ clipId, hotkey }) => <HotkeyRow
                key={clipId}
                label={translate('component.settingsHotkeysPanel.clipNumber', { number: clipId })}
                value={hotkey}
                onChange={async (nextHotkey) => {
                  try {
                    await invoke('update_clip_hotkey', { clipId, hotkey: nextHotkey });
                  } catch (error) {
                    console.error('Failed to update clip hotkey:', error);
                    showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.thatClipHotkeyCouldNotBeRegistered'); } });
                  }
                }}
              />)}
        </div>
      </section>}

      {settings.enableTransformations && manualTransforms.length > 0 && <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">{translate('component.settingsHotkeysPanel.savedTransformHotkeys')}{manualTransforms.length})</h4>
        <div className="theme-surface overlay-scroll-region max-h-60 overflow-y-auto rounded-xl border">
          {manualTransforms.map((manualTransform) => <HotkeyRow key={manualTransform.id} label={manualTransform.name} value={manualTransform.hotkey ?? null} onChange={async (hotkey) => {
              try {
                await transformsApi.updateManualHotkey(manualTransform.stableRef, hotkey);
                onRefreshManualTransforms?.();
              } catch (error) {
                console.error('Failed to update Advanced Transform hotkey:', error);
                showToast({ tone: 'error', get message() { return translate('component.settingsHotkeysPanel.thatAdvancedTransformHotkeyCouldNotBeRegistered'); } });
              }
            }} />)}
        </div>
      </section>}

      <section className="space-y-2">
        <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">{translate('component.settingsHotkeysPanel.pasteClipsByPosition')}</h4>
        <div className="theme-surface overflow-hidden rounded-xl border">
          {Array.from({ length: 9 }, (_, index) => index + 1).map((number) => {
            const key = `pasteClip${number}Hotkey` as HotkeySetting;
            return <HotkeyRow key={key} label={translate('component.settingsHotkeysPanel.pasteClipNumber', { number: number })} value={(settings[key] as string) || null} onChange={(value) => void updateSettingHotkey(key, value)} />;
          })}
        </div>
      </section>
    </div>
  );
}
