import { useCallback, useEffect, type Dispatch, type SetStateAction } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';

interface UseCopyQueueControllerOptions {
  enabled: boolean;
  active: boolean;
  navigateToTab: (route: string) => void;
  setSelectedBinId: Dispatch<SetStateAction<number | null>>;
  refreshStatus: () => Promise<void>;
}

export function useCopyQueueController({
  enabled,
  active,
  navigateToTab,
  setSelectedBinId,
  refreshStatus,
}: UseCopyQueueControllerOptions) {
  const toggleCopyQueue = useCallback(async () => {
    if (!enabled) return;
    try {
      if (active) await invoke('stop_sequential_paste');
      else {
        await invoke('start_sequential_paste');
        navigateToTab('sequential');
        setSelectedBinId(null);
      }
      await refreshStatus();
    } catch (error) {
      console.error('Failed to toggle copy queue:', error);
    }
  }, [active, enabled, navigateToTab, refreshStatus, setSelectedBinId]);

  useEffect(() => {
    if (!enabled && active) void invoke('stop_sequential_paste').then(refreshStatus).catch(console.error);
  }, [active, enabled, refreshStatus]);

  return toggleCopyQueue;
}
