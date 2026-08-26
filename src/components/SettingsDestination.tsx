import type { useAppController } from '../hooks/useAppController';
import { SettingsModal } from './AppDestinations';
import { ocrStatusSearchQuery } from './ocrStatusModel';

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
  />;
}
