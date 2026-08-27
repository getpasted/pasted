import React, { useEffect } from 'react';
import { Settings } from 'lucide-react';
import { AppSettings, BlacklistApp, ManualTransform, Bin, type ClipSearchRequest } from '../types';
import { SettingsTabs, type SettingsTab } from './SettingsTabs';
import { SettingsBlacklistPanel } from './SettingsBlacklistPanel';
import { SettingsGeneralPanel } from './SettingsGeneralPanel';
import { SettingsHotkeysPanel } from './SettingsHotkeysPanel';
import { SettingsSyncPanel } from './SettingsSyncPanel';
import { ToolPageHeader } from './ToolPageHeader';
import { IntelligenceConnectionsPanel } from './IntelligenceConnectionsPanel';
import { SettingsFeaturesPanel } from './SettingsFeaturesPanel';
import { SettingsAboutPanel } from './SettingsAboutPanel';
import { SettingsResetPanel } from './SettingsResetPanel';
import { SettingsNotificationsPanel } from './SettingsNotificationsPanel';
import { SettingsAnalysisPanel } from './SettingsAnalysisPanel';
import { SettingsWelcomePanel } from './SettingsWelcomePanel';
import { SettingsSecurityPanel } from './SettingsSecurityPanel';
import { SettingsSearchHistoryPanel } from './SettingsSearchHistoryPanel';
import { translate } from '../localization/runtime';

interface SettingsModalProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  blacklistApps: BlacklistApp[];
  onAddBlacklistApp: (appName: string) => void;
  onRemoveBlacklistApp: (appId: string) => void;
  onToggleBlacklistRule: (appId: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreFiles' | 'ignoreHotkeys') => void;
  onResetBlacklistApps: () => void;
  manualTransforms?: ManualTransform[];
  onRefreshManualTransforms?: () => void;
  bins?: Bin[];
  onRefreshBins?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
  onResetClientState?: (resetInPlace: boolean) => void;
  onClearHistory?: (permanent: boolean) => void;
  onRestoreAllTrashedClips?: () => Promise<number>;
  trashedClipCount?: number;
  onResetColumnWidths?: () => void;
  activeTab: SettingsTab;
  onActiveTabChange: (tab: SettingsTab) => void;
  onOpenAnalytics?: () => void;
  onSearchClips: (clipIds: number[]) => void;
  onRunSearch: (request: ClipSearchRequest) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  settings,
  onUpdateSettings,
  blacklistApps,
  onAddBlacklistApp,
  onRemoveBlacklistApp,
  onToggleBlacklistRule,
  onResetBlacklistApps,
  manualTransforms = [],
  onRefreshManualTransforms,
  bins = [],
  onRefreshBins,
  onRefreshClips,
  onRefreshTrashedClips,
  onResetClientState,
  onClearHistory,
  onRestoreAllTrashedClips,
  trashedClipCount = 0,
  onResetColumnWidths,
  activeTab,
  onActiveTabChange,
  onOpenAnalytics,
  onSearchClips,
  onRunSearch,
}) => {
  useEffect(() => {
    if (!settings.enableNotifications && activeTab === 'notifications') {
      onActiveTabChange('functionality');
    }
  }, [activeTab, onActiveTabChange, settings.enableNotifications]);

  useEffect(() => {
    if (!settings.enableAppLock && activeTab === 'security') {
      onActiveTabChange('functionality');
    }
  }, [activeTab, onActiveTabChange, settings.enableAppLock]);

  useEffect(() => {
    if (!settings.enableHotkeys && activeTab === 'hotkeys') {
      onActiveTabChange('functionality');
    }
  }, [activeTab, onActiveTabChange, settings.enableHotkeys]);

  useEffect(() => {
    if (!settings.enableSearch && activeTab === 'search-history') {
      onActiveTabChange('functionality');
    }
  }, [activeTab, onActiveTabChange, settings.enableSearch]);

  return (
    <div className="tools-page settings-page flex-1 settings-modal-bg h-screen overflow-hidden font-sans select-none flex flex-col">
      <ToolPageHeader
        icon={<Settings className="w-4 h-4" />}
        title={translate('destination.settings')}
        actions={<SettingsTabs activeTab={activeTab} onChange={onActiveTabChange} showNotifications={settings.enableNotifications} showSecurity={settings.enableAppLock} showHotkeys={settings.enableHotkeys} showSearchHistory={settings.enableSearch} />}
      />

      <div data-pasted-scroll-key={`settings:${activeTab}`} className="tools-scroll-region flex-1 overflow-y-auto p-6">
        <div className={`w-full mx-auto max-w-xl ${activeTab === 'storage' || activeTab === 'general' ? 'space-y-4' : 'settings-primary-well theme-panel rounded-2xl border p-6'}`}>

        {/* TAB 1: GENERAL */}
        {activeTab === 'general' && (
          <>
            <div className="settings-primary-well theme-panel rounded-2xl border p-6">
              <SettingsGeneralPanel
                settings={settings}
                onUpdateSettings={onUpdateSettings}
                onClearHistory={onClearHistory}
                onRestoreAllTrashedClips={onRestoreAllTrashedClips}
                trashedClipCount={trashedClipCount}
                onResetColumnWidths={onResetColumnWidths}
              />
            </div>
            <SettingsWelcomePanel onOpen={() => onUpdateSettings({ onboardingVersion: 0 })} />
          </>
        )}

        {activeTab === 'functionality' && (
          <SettingsFeaturesPanel settings={settings} onUpdateSettings={onUpdateSettings} />
        )}

        {settings.enableSearch && activeTab === 'search-history' && (
          <SettingsSearchHistoryPanel onRunSearch={onRunSearch} />
        )}

        {activeTab === 'analysis' && (
          <SettingsAnalysisPanel
            contentClassificationEnabled={settings.enableContentClassification}
            fileFormatsEnabled={settings.enableFileFormats}
            ocrEnabled={settings.enableOcr}
            transcriptionsEnabled={settings.enableTranscriptions}
            transformationsEnabled={settings.enableTransformations}
            typesEnabled={settings.enableTypes}
            sourcesEnabled={settings.enableSources}
            searchEnabled={settings.enableSearch}
            onOpenIntelligence={() => onActiveTabChange('intelligence')}
            onSearchClips={onSearchClips}
          />
        )}

        {settings.enableNotifications && activeTab === 'notifications' && (
          <SettingsNotificationsPanel settings={settings} onUpdateSettings={onUpdateSettings} />
        )}

        {settings.enableAppLock && activeTab === 'security' && <SettingsSecurityPanel />}

        {/* HOTKEYS */}
        {settings.enableHotkeys && activeTab === 'hotkeys' && (
          <SettingsHotkeysPanel
            settings={settings}
            bins={bins}
            manualTransforms={manualTransforms}
            onUpdateSettings={onUpdateSettings}
            onRefreshBins={onRefreshBins}
            onRefreshManualTransforms={onRefreshManualTransforms}
          />
        )}

        {/* INTELLIGENCE */}
        {activeTab === 'intelligence' && <IntelligenceConnectionsPanel />}

        {/* APP EXCLUSIONS */}
        {activeTab === 'app-exclusions' && (
          <SettingsBlacklistPanel
            apps={blacklistApps}
            onAddApp={onAddBlacklistApp}
            onRemoveApp={onRemoveBlacklistApp}
            onToggleRule={onToggleBlacklistRule}
            onResetApps={onResetBlacklistApps}
            settings={settings}
            onUpdateSettings={onUpdateSettings}
          />
        )}

        {/* STORAGE */}
        {activeTab === 'storage' && (
          <>
            <div className="settings-primary-well theme-panel rounded-2xl border p-6">
              <SettingsSyncPanel
                onRefreshBins={onRefreshBins}
                onRefreshManualTransforms={onRefreshManualTransforms}
                onRefreshClips={onRefreshClips}
                onRefreshTrashedClips={onRefreshTrashedClips}
                analyticsEnabled={settings.enableAnalytics}
                activityEnabled={settings.enableActivityLog}
                onOpenAnalytics={onOpenAnalytics}
              />
            </div>
            <SettingsResetPanel
              onRefreshBins={onRefreshBins}
              onRefreshManualTransforms={onRefreshManualTransforms}
              onRefreshClips={onRefreshClips}
              onRefreshTrashedClips={onRefreshTrashedClips}
              onResetClientState={onResetClientState}
            />
          </>
        )}

        {activeTab === 'about' && <SettingsAboutPanel updateChecksEnabled={settings.enableUpdateChecks} />}
        </div>
      </div>
    </div>
  );
};
