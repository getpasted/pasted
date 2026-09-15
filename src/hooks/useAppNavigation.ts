import { useCallback, useEffect, useRef, useState } from 'react';
import { APP_EVENTS } from '../utils/appEvents';
import { resolveAppNavigationTarget } from '../utils/appNavigation';
import { featureForRoute, type FeatureId } from '../utils/features';
import {
  writeAppUiState,
  type AppUiState,
  type SidebarSectionId,
} from '../utils/appUiState';
import { wasBackupClientStateRestoredBeforeMount } from '../utils/backupClientState';
import type { Bin } from '../types';
import { useAppEvent } from './useAppEvent';
import { useSearchDialogController } from './useSearchDialogController';

interface UseAppNavigationOptions {
  restoredUiState: AppUiState;
  enabledFeatures: Record<FeatureId, boolean>;
  bins: Bin[];
  startupView: string;
  settingsHydrated: boolean;
  initialDataLoaded: boolean;
  selectedClipId: number | null;
}

export function useAppNavigation({
  restoredUiState,
  enabledFeatures,
  bins,
  startupView,
  settingsHydrated,
  initialDataLoaded,
  selectedClipId,
}: UseAppNavigationOptions) {
  const [currentTab, setCurrentTab] = useState(restoredUiState.currentTab);
  const [activeSettingsTab, setActiveSettingsTab] = useState(restoredUiState.settingsTab);
  const [activeHelpTopic, setActiveHelpTopic] = useState(restoredUiState.helpTopic);
  const [activeTransformWorkspace, setActiveTransformWorkspace] = useState(restoredUiState.transformWorkspace);
  const [selectedBinId, setSelectedBinId] = useState<number | null>(restoredUiState.selectedBinId);
  const [searchQuery, setSearchQuery] = useState('');
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(restoredUiState.isSidebarCollapsed);
  const [sidebarSections, setSidebarSections] = useState(restoredUiState.sidebarSections);
  const startupViewAppliedRef = useRef(false);
  const preserveRestoredViewRef = useRef(wasBackupClientStateRestoredBeforeMount());
  const searchDialog = useSearchDialogController({
    enabled: enabledFeatures.search,
    currentTab,
    committedQuery: searchQuery,
    setCommittedQuery: setSearchQuery,
    setCurrentTab,
    setSelectedBinId,
  });

  const handleSidebarSectionStateChange = useCallback((section: SidebarSectionId, open: boolean) => {
    setSidebarSections((previous) => previous[section] === open
      ? previous
      : { ...previous, [section]: open });
  }, []);

  const navigateToTab = useCallback((route: string) => {
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    if (route === 'search') {
      searchDialog.openSearchDialog();
      return;
    }
    const requiredFeature = featureForRoute(route);
    const target = resolveAppNavigationTarget(
      requiredFeature && !enabledFeatures[requiredFeature] ? 'all' : route,
    );
    if (target.settingsTab) setActiveSettingsTab(target.settingsTab);
    if (target.helpTopic) setActiveHelpTopic(target.helpTopic);
    if (target.transformWorkspace) setActiveTransformWorkspace(target.transformWorkspace);
    setCurrentTab(target.tab);
    if (target.tab !== 'bin') setSelectedBinId(null);
  }, [enabledFeatures, searchDialog.openSearchDialog]);

  useEffect(() => {
    const requiredFeature = featureForRoute(currentTab);
    if (requiredFeature && !enabledFeatures[requiredFeature]) {
      setCurrentTab('all');
      setSelectedBinId(null);
    }
  }, [currentTab, enabledFeatures]);

  useEffect(() => {
    if (!settingsHydrated || startupViewAppliedRef.current) return;
    startupViewAppliedRef.current = true;
    if (startupView === 'clip_history' && !preserveRestoredViewRef.current) {
      setCurrentTab('all');
      setSelectedBinId(null);
    }
  }, [settingsHydrated, startupView]);

  useEffect(() => {
    if (!settingsHydrated || !initialDataLoaded) return;
    if (currentTab === 'bin' && (selectedBinId === null || !bins.some((bin) => bin.id === selectedBinId))) {
      setCurrentTab('all');
      setSelectedBinId(null);
    }
  }, [bins, currentTab, initialDataLoaded, selectedBinId, settingsHydrated]);

  useAppEvent<string>(APP_EVENTS.navigateTab, navigateToTab);
  useAppEvent<number>(APP_EVENTS.navigateBin, (binId) => {
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    setSelectedBinId(binId);
    setCurrentTab('bin');
  });

  useEffect(() => {
    if (!settingsHydrated || !initialDataLoaded) return;
    writeAppUiState({
      version: 2,
      currentTab,
      settingsTab: activeSettingsTab,
      helpTopic: activeHelpTopic,
      transformWorkspace: activeTransformWorkspace,
      selectedBinId: currentTab === 'bin' ? selectedBinId : null,
      selectedClipId,
      isSidebarCollapsed,
      sidebarSections,
    });
  }, [
    activeHelpTopic,
    activeSettingsTab,
    activeTransformWorkspace,
    currentTab,
    initialDataLoaded,
    isSidebarCollapsed,
    selectedBinId,
    selectedClipId,
    settingsHydrated,
    sidebarSections,
  ]);

  return {
    currentTab,
    setCurrentTab,
    activeSettingsTab,
    setActiveSettingsTab,
    activeHelpTopic,
    setActiveHelpTopic,
    activeTransformWorkspace,
    setActiveTransformWorkspace,
    selectedBinId,
    setSelectedBinId,
    searchQuery,
    isSidebarCollapsed,
    setIsSidebarCollapsed,
    sidebarSections,
    handleSidebarSectionStateChange,
    navigateToTab,
    ...searchDialog,
  };
}
