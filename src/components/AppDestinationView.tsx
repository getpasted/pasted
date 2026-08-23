import type { ReactNode } from 'react';
import type { useAppController } from '../hooks/useAppController';
import { ActivityLogView, AnalyticsView, HelpView, SettingsModal, TransformationsView } from './AppDestinations';

type AppController = ReturnType<typeof useAppController>;

export function AppDestinationView({
  controller,
  renderClipWorkspace,
}: {
  controller: AppController;
  renderClipWorkspace: () => ReactNode;
}) {
  const { shell, settings, data, navigation, layout, handlers } = controller;
  const { appSettings } = shell;
  const {
    blacklistApps,
    handleUpdateSettings,
    handleAddBlacklistApp,
    handleRemoveBlacklistApp,
    handleToggleBlacklistRule,
  } = settings;
  const {
    bins,
    manualTransforms,
    trashedClips,
    fetchBins,
    fetchClips,
    fetchTrashedClips,
    fetchManualTransforms,
  } = data;
  const {
    currentTab,
    activeSettingsTab,
    setActiveSettingsTab,
    activeHelpTopic,
    setActiveHelpTopic,
    activeTransformWorkspace,
    setActiveTransformWorkspace,
  } = navigation;
  const { resetColumnWidths } = layout;
  const { handleSidebarNavigate, handleRestoreAllTrashedClips } = handlers;

  if (currentTab === 'transformations') {
    return <TransformationsView
      manualTransforms={manualTransforms}
      onRefreshManualTransforms={fetchManualTransforms}
      activeWorkspace={activeTransformWorkspace}
      onActiveWorkspaceChange={setActiveTransformWorkspace}
    />;
  }
  if (currentTab === 'activity') return <ActivityLogView />;
  if (currentTab === 'analytics') return <AnalyticsView />;
  if (currentTab === 'help') {
    return <HelpView activeTopic={activeHelpTopic} onActiveTopicChange={setActiveHelpTopic} />;
  }
  if (currentTab !== 'settings') return renderClipWorkspace();

  return <SettingsModal
    settings={appSettings}
    onUpdateSettings={handleUpdateSettings}
    blacklistApps={blacklistApps}
    onAddBlacklistApp={handleAddBlacklistApp}
    onRemoveBlacklistApp={handleRemoveBlacklistApp}
    onToggleBlacklistRule={handleToggleBlacklistRule}
    onRefreshManualTransforms={fetchManualTransforms}
    bins={bins}
    onRefreshBins={fetchBins}
    onRefreshClips={fetchClips}
    onRefreshTrashedClips={fetchTrashedClips}
    onClearHistory={(permanent) => controller.overlays.setClearHistoryMode(permanent ? 'purge' : 'trash')}
    onRestoreAllTrashedClips={handleRestoreAllTrashedClips}
    trashedClipCount={trashedClips.length}
    onResetColumnWidths={resetColumnWidths}
    activeTab={activeSettingsTab}
    onActiveTabChange={setActiveSettingsTab}
    onOpenAnalytics={() => handleSidebarNavigate('analytics')}
  />;
}
