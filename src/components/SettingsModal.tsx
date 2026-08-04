import React, { useState } from 'react';
import { Settings } from 'lucide-react';
import { AppSettings, BlacklistApp, Pipeline, Bin } from '../types';
import { SettingsTabs, type SettingsTab } from './SettingsTabs';
import { SettingsBlacklistPanel } from './SettingsBlacklistPanel';
import { SettingsGeneralPanel } from './SettingsGeneralPanel';
import { SettingsHotkeysPanel } from './SettingsHotkeysPanel';
import { SettingsSyncPanel } from './SettingsSyncPanel';
import { ToolPageHeader } from './ToolPageHeader';
import { IntelligenceConnectionsPanel } from './IntelligenceConnectionsPanel';

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
  onClearHistory?: (permanent: boolean) => void;
  onResetColumnWidths?: () => void;
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
  onClearHistory,
  onResetColumnWidths,
}) => {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  return (
    <div className="tools-page settings-page flex-1 settings-modal-bg h-screen overflow-hidden font-sans select-none flex flex-col">
      <ToolPageHeader
        icon={<Settings className="w-4 h-4" />}
        title="Settings"
        actions={<SettingsTabs activeTab={activeTab} onChange={setActiveTab} />}
      />

      <div className="tools-scroll-region flex-1 overflow-y-auto p-6">
        <div className="w-full max-w-xl mx-auto space-y-6">

        {/* TAB 1: GENERAL */}
        {activeTab === 'general' && (
          <SettingsGeneralPanel
            settings={settings}
            onUpdateSettings={onUpdateSettings}
            onClearHistory={onClearHistory}
            onResetColumnWidths={onResetColumnWidths}
          />
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
        {activeTab === 'connections' && <IntelligenceConnectionsPanel />}

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
        {activeTab === 'sync' && (
          <SettingsSyncPanel
            onRefreshBins={onRefreshBins}
            onRefreshPipelines={onRefreshPipelines}
            onRefreshClips={onRefreshClips}
          />
        )}
        </div>
      </div>
    </div>
  );
};
