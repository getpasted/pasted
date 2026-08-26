import type { useAppController } from '../hooks/useAppController';
import { SettingsModal } from './AppDestinations';
import { ocrStatusSearchQuery } from './ocrStatusModel';
import { DEFAULT_APP_UI_STATE, SIDEBAR_SECTION_IDS } from '../utils/appUiState';
import { searchHistoryRequestQuery } from '../utils/searchHistory';

type AppController = ReturnType<typeof useAppController>;

export function SettingsDestination({ controller }: { controller: AppController }) {
  const { shell, settings, data, navigation, layout, handlers } = controller;
  return <SettingsModal
    settings={shell.appSettings}
    onUpdateSettings={settings.handleUpdateSettings}
    blacklistApps={settings.blacklistApps}
    onAddBlacklistApp={settings.handleAddBlacklistApp}
    onRemoveBlacklistApp={settings.handleRemoveBlacklistApp}
    onToggleBlacklistRule={settings.handleToggleBlacklistRule}
    onResetBlacklistApps={settings.handleResetBlacklistApps}
    onRefreshManualTransforms={data.fetchManualTransforms}
    bins={data.bins}
    onRefreshBins={data.fetchBins}
    onRefreshClips={data.fetchClips}
    onRefreshTrashedClips={data.fetchTrashedClips}
    onResetClientState={(resetInPlace) => {
      settings.prepareForFactoryReset(resetInPlace);
      if (!resetInPlace) return;
      navigation.setCurrentTab(DEFAULT_APP_UI_STATE.currentTab);
      navigation.setActiveSettingsTab(DEFAULT_APP_UI_STATE.settingsTab);
      navigation.setActiveHelpTopic(DEFAULT_APP_UI_STATE.helpTopic);
      navigation.setActiveTransformWorkspace(DEFAULT_APP_UI_STATE.transformWorkspace);
      navigation.setSelectedBinId(null);
      navigation.setSearchQuery('');
      navigation.setIsSidebarCollapsed(DEFAULT_APP_UI_STATE.isSidebarCollapsed);
      for (const section of SIDEBAR_SECTION_IDS) {
        navigation.handleSidebarSectionStateChange(section, DEFAULT_APP_UI_STATE.sidebarSections[section]);
      }
      layout.resetColumnWidths();
      controller.selection.clearClipSelection();
    }}
    onClearHistory={(permanent) => controller.overlays.setClearHistoryMode(permanent ? 'purge' : 'trash')}
    onRestoreAllTrashedClips={handlers.handleRestoreAllTrashedClips}
    trashedClipCount={data.trashedClips.length}
    onResetColumnWidths={layout.resetColumnWidths}
    activeTab={navigation.activeSettingsTab}
    onActiveTabChange={navigation.setActiveSettingsTab}
    onOpenAnalytics={() => handlers.handleSidebarNavigate('analytics')}
    onSearchClips={(clipIds) => {
      navigation.setSearchQuery(ocrStatusSearchQuery(clipIds));
      handlers.handleSidebarNavigate('search');
    }}
    onRunSearch={(request) => {
      const query = searchHistoryRequestQuery(request);
      if (query === null) return;
      navigation.setSearchQuery(query);
      handlers.handleSidebarNavigate('search');
    }}
  />;
}
