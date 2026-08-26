import type { ReactNode } from 'react';
import type { useAppController } from '../hooks/useAppController';
import { ActivityLogView, AnalyticsView, HelpView, TransformationsView } from './AppDestinations';
import { SettingsDestination } from './SettingsDestination';

type AppController = ReturnType<typeof useAppController>;

export function AppDestinationView({
  controller,
  renderClipWorkspace,
}: {
  controller: AppController;
  renderClipWorkspace: () => ReactNode;
}) {
  const { data, navigation } = controller;
  const { manualTransforms, fetchManualTransforms } = data;
  const {
    currentTab,
    activeHelpTopic,
    setActiveHelpTopic,
    activeTransformWorkspace,
    setActiveTransformWorkspace,
  } = navigation;
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

  return <SettingsDestination controller={controller} />;
}
