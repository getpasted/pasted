import { useCallback, useEffect, useRef, useState } from 'react';
import { APP_EVENTS } from '../utils/appEvents';
import {
  isClipCollectionRoute,
  resolveAppNavigationTarget,
  resolveSearchExit,
  type ClipViewLocation,
} from '../utils/appNavigation';
import { featureForRoute, type FeatureId } from '../utils/features';
import {
  writeAppUiState,
  type AppUiState,
  type SidebarSectionId,
} from '../utils/appUiState';
import type { Bin } from '../types';
import { useAppEvent } from './useAppEvent';

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
  const lastClipViewRef = useRef<ClipViewLocation>({
    tab: restoredUiState.currentTab,
    binId: restoredUiState.selectedBinId,
  });

  const handleSidebarSectionStateChange = useCallback((section: SidebarSectionId, open: boolean) => {
    setSidebarSections((previous) => previous[section] === open
      ? previous
      : { ...previous, [section]: open });
  }, []);

  const navigateToTab = useCallback((route: string) => {
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    const requiredFeature = featureForRoute(route);
    const target = resolveAppNavigationTarget(
      requiredFeature && !enabledFeatures[requiredFeature] ? 'all' : route,
    );
    if (target.settingsTab) setActiveSettingsTab(target.settingsTab);
    if (target.helpTopic) setActiveHelpTopic(target.helpTopic);
    if (target.transformWorkspace) setActiveTransformWorkspace(target.transformWorkspace);
    setCurrentTab(target.tab);
    if (target.tab !== 'bin') setSelectedBinId(null);
    if (target.tab === 'search') {
      requestAnimationFrame(() => {
        document.querySelector<HTMLInputElement>('[data-sidebar-search-input]')?.focus();
      });
    }
  }, [enabledFeatures]);

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
    if (startupView === 'clip_history') {
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

  const enterSearchView = useCallback(() => {
    if (currentTab !== 'search') setCurrentTab('search');
  }, [currentTab]);

  useEffect(() => {
    if (isClipCollectionRoute(currentTab)) {
      lastClipViewRef.current = { tab: currentTab, binId: currentTab === 'bin' ? selectedBinId : null };
    }
  }, [currentTab, selectedBinId]);

  const exitEmptySearch = useCallback(() => {
    const target = resolveSearchExit(lastClipViewRef.current, new Set(bins.map(({ id }) => id)));
    setSelectedBinId(target.binId);
    setCurrentTab(target.tab);
  }, [bins]);

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
    setSearchQuery,
    isSidebarCollapsed,
    setIsSidebarCollapsed,
    sidebarSections,
    handleSidebarSectionStateChange,
    navigateToTab,
    enterSearchView,
    exitEmptySearch,
  };
}
