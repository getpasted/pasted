import React, { useState } from 'react';
import { AppSettings, BlacklistApp, FilterRule, Board } from '../types';
import { SettingsTabs, type SettingsTab } from './SettingsTabs';
import { SettingsBlacklistPanel } from './SettingsBlacklistPanel';
import { SettingsGeneralPanel } from './SettingsGeneralPanel';
import { SettingsHotkeysPanel } from './SettingsHotkeysPanel';
import { SettingsSyncPanel } from './SettingsSyncPanel';

interface SettingsModalProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  blacklistApps: BlacklistApp[];
  onAddBlacklistApp: (appName: string) => void;
  onRemoveBlacklistApp: (appId: string) => void;
  onToggleBlacklistRule: (appId: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreShortcuts') => void;
  filters?: FilterRule[];
  onRefreshFilters?: () => void;
  boards?: Board[];
  onRefreshBoards?: () => void;
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
  filters = [],
  onRefreshFilters,
  boards = [],
  onRefreshBoards,
  onRefreshClips,
  onClearHistory,
  onResetColumnWidths,
}) => {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  return (
    <div className="tools-page settings-page flex-1 settings-modal-bg h-screen overflow-y-auto bg-[#141414] text-gray-100 font-sans select-none flex flex-col items-center p-6">
      <div className="w-full max-w-xl space-y-6">
        <SettingsTabs activeTab={activeTab} onChange={setActiveTab} />

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
            boards={boards}
            filters={filters}
            onUpdateSettings={onUpdateSettings}
            onRefreshBoards={onRefreshBoards}
            onRefreshFilters={onRefreshFilters}
          />
        )}

        {/* TAB 3: BLACKLIST */}
        {activeTab === 'blacklist' && (
          <SettingsBlacklistPanel
            apps={blacklistApps}
            onAddApp={onAddBlacklistApp}
            onRemoveApp={onRemoveBlacklistApp}
            onToggleRule={onToggleBlacklistRule}
          />
        )}

        {/* TAB 4: SYNC & BACKUP */}
        {activeTab === 'sync' && (
          <SettingsSyncPanel
            onRefreshBoards={onRefreshBoards}
            onRefreshFilters={onRefreshFilters}
            onRefreshClips={onRefreshClips}
          />
        )}
      </div>
    </div>
  );
};
