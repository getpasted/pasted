import React, { useEffect, useState } from 'react';
import { Settings } from 'lucide-react';
import { AppSettings, BlacklistApp, Pipeline, Bin } from '../types';
import { SettingsTabs, type SettingsTab } from './SettingsTabs';
import { SettingsBlacklistPanel } from './SettingsBlacklistPanel';
import { SettingsGeneralPanel } from './SettingsGeneralPanel';
import { SettingsHotkeysPanel } from './SettingsHotkeysPanel';
import { SettingsSyncPanel } from './SettingsSyncPanel';
import { ToolPageHeader } from './ToolPageHeader';
import { IntelligenceConnectionsPanel } from './IntelligenceConnectionsPanel';
import { SettingsDebugPanel } from './SettingsDebugPanel';
import { SettingsFeaturesPanel } from './SettingsFeaturesPanel';
import { SettingsAboutPanel } from './SettingsAboutPanel';
import { SettingsResetPanel } from './SettingsResetPanel';

interface SettingsModalProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  blacklistApps: BlacklistApp[];
  onAddBlacklistApp: (appName: string) => void;
  onRemoveBlacklistApp: (appId: string) => void;
  onToggleBlacklistRule: (appId: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreShortcuts') => void;
  pipelines?: Pipeline[];
  onRefreshPipelines?: () => void;
  bins?: Bin[];
  onRefreshBins?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
  onClearHistory?: (permanent: boolean) => void;
  onResetColumnWidths?: () => void;
  requestedTab?: SettingsTab;
  navigationKey?: number;
  onOpenAnalytics?: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  settings,
  onUpdateSettings,
  blacklistApps,
  onAddBlacklistApp,
  onRemoveBlacklistApp,
  onToggleBlacklistRule,
  pipelines = [],
  onRefreshPipelines,
  bins = [],
  onRefreshBins,
  onRefreshClips,
  onRefreshTrashedClips,
  onClearHistory,
  onResetColumnWidths,
  requestedTab,
  navigationKey,
  onOpenAnalytics,
}) => {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');

  useEffect(() => {
    if (requestedTab) setActiveTab(requestedTab);
  }, [navigationKey, requestedTab]);

  useEffect(() => {
    if (!settings.enableTransformations && activeTab === 'connections') {
      setActiveTab('features');
    }
  }, [activeTab, settings.enableTransformations]);

  useEffect(() => {
    if (!settings.enableDiagnostics && activeTab === 'diagnostics') {
      setActiveTab('features');
    }
  }, [activeTab, settings.enableDiagnostics]);

  return (
    <div className="tools-page settings-page flex-1 settings-modal-bg h-screen overflow-hidden font-sans select-none flex flex-col">
      <ToolPageHeader
        icon={<Settings className="w-4 h-4" />}
        title="Settings"
        actions={<SettingsTabs activeTab={activeTab} onChange={setActiveTab} showConnections={settings.enableTransformations} showDiagnostics={settings.enableDiagnostics} />}
      />

      <div className="tools-scroll-region flex-1 overflow-y-auto p-6">
        <div className={`w-full max-w-xl mx-auto ${activeTab === 'storage' ? 'space-y-4' : 'settings-primary-well theme-panel rounded-2xl border p-6'}`}>

        {/* TAB 1: GENERAL */}
        {activeTab === 'general' && (
          <SettingsGeneralPanel
            settings={settings}
            onUpdateSettings={onUpdateSettings}
            onClearHistory={onClearHistory}
            onResetColumnWidths={onResetColumnWidths}
          />
        )}

        {activeTab === 'features' && (
          <SettingsFeaturesPanel settings={settings} onUpdateSettings={onUpdateSettings} />
        )}

        {/* TAB 2: HOTKEYS */}
        {activeTab === 'hotkeys' && (
          <SettingsHotkeysPanel
            settings={settings}
            bins={bins}
            pipelines={pipelines}
            onUpdateSettings={onUpdateSettings}
            onRefreshBins={onRefreshBins}
            onRefreshPipelines={onRefreshPipelines}
          />
        )}

        {/* TAB 3: CONNECTIONS */}
        {settings.enableTransformations && activeTab === 'connections' && <IntelligenceConnectionsPanel />}

        {/* TAB 4: BLACKLIST */}
        {activeTab === 'blacklist' && (
          <SettingsBlacklistPanel
            apps={blacklistApps}
            onAddApp={onAddBlacklistApp}
            onRemoveApp={onRemoveBlacklistApp}
            onToggleRule={onToggleBlacklistRule}
          />
        )}

        {/* TAB 5: SYNC & BACKUP */}
        {activeTab === 'storage' && (
          <>
            <div className="settings-primary-well theme-panel rounded-2xl border p-6">
              <SettingsSyncPanel
                onRefreshBins={onRefreshBins}
                onRefreshPipelines={onRefreshPipelines}
                onRefreshClips={onRefreshClips}
                onRefreshTrashedClips={onRefreshTrashedClips}
                analyticsEnabled={settings.enableAnalytics}
                onOpenAnalytics={onOpenAnalytics}
              />
            </div>
            <SettingsResetPanel
              onRefreshBins={onRefreshBins}
              onRefreshPipelines={onRefreshPipelines}
              onRefreshClips={onRefreshClips}
              onRefreshTrashedClips={onRefreshTrashedClips}
            />
          </>
        )}

        {/* TAB 6: DIAGNOSTICS */}
        {settings.enableDiagnostics && activeTab === 'diagnostics' && <SettingsDebugPanel ocrEnabled={settings.enableOcr} />}

        {activeTab === 'about' && <SettingsAboutPanel />}
        </div>
      </div>
    </div>
  );
};
