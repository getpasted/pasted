import type { Dispatch, SetStateAction } from 'react';
import type { AppSettings, Bin, ClipItem } from '../types';
import { APP_EVENTS } from '../utils/appEvents';
import { ACTUAL_SIZE, stepAppZoom } from '../utils/appZoom';
import type { FeatureId } from '../utils/features';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { useAppEvent } from './useAppEvent';

interface UseAppMenuActionsOptions {
  enabled: boolean;
  enabledFeatures: Record<FeatureId, boolean>;
  selectedClip: ClipItem | null;
  selectedClipIds: Set<number>;
  selectedClipViewPolicy: ClipViewPolicy;
  textSize: number;
  setIsBinModalOpen: Dispatch<SetStateAction<boolean>>;
  setEditingBin: Dispatch<SetStateAction<Bin | null>>;
  setIsSidebarCollapsed: Dispatch<SetStateAction<boolean>>;
  updateSettings: (updates: Partial<AppSettings>) => void;
  toggleClipboardPause: () => unknown;
  toggleCopyQueue: () => unknown;
  copyClip: (clip: ClipItem) => unknown;
  promptAddNote: (clip: ClipItem) => unknown;
  promptNameClip: (clip: ClipItem) => unknown;
  togglePin: (clipId: number) => unknown;
  toggleProtected: (clipId: number) => unknown;
  batchTrash: () => unknown;
  purgeClipPermanently: (clipId: number) => unknown;
  deleteClip: (clipId: number) => unknown;
  resetColumnWidths: () => unknown;
  refreshData: () => Promise<unknown>;
}

export function useAppMenuActions({
  enabled,
  enabledFeatures,
  selectedClip,
  selectedClipIds,
  selectedClipViewPolicy,
  textSize,
  setIsBinModalOpen,
  setEditingBin,
  setIsSidebarCollapsed,
  updateSettings,
  toggleClipboardPause,
  toggleCopyQueue,
  copyClip,
  promptAddNote,
  promptNameClip,
  togglePin,
  toggleProtected,
  batchTrash,
  purgeClipPermanently,
  deleteClip,
  resetColumnWidths,
  refreshData,
}: UseAppMenuActionsOptions) {
  useAppEvent<string>(APP_EVENTS.appMenuAction, (action) => {
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    switch (action) {
      case 'new-bin':
        if (!enabledFeatures.bins) break;
        setEditingBin(null);
        setIsBinModalOpen(true);
        break;
      case 'toggle-history':
        void toggleClipboardPause();
        break;
      case 'toggle-queue':
        if (enabledFeatures.queue) void toggleCopyQueue();
        break;
      case 'copy-selected-clip':
        if (selectedClip) void copyClip(selectedClip);
        break;
      case 'add-note':
        if (enabledFeatures.notes && selectedClip && selectedClipViewPolicy.canEditNotes) promptAddNote(selectedClip);
        break;
      case 'name-clip':
        if (enabledFeatures.naming && selectedClip && selectedClipViewPolicy.canOrganize) promptNameClip(selectedClip);
        break;
      case 'toggle-pin':
        if (enabledFeatures.pinning && selectedClip && selectedClipViewPolicy.canOrganize) togglePin(selectedClip.id);
        break;
      case 'toggle-protection':
        if (enabledFeatures.protection && selectedClip && selectedClipViewPolicy.canOrganize) toggleProtected(selectedClip.id);
        break;
      case 'trash-selected':
        if (selectedClipIds.size > 1) {
          void batchTrash();
        } else if (selectedClip) {
          if (selectedClipViewPolicy.state === 'trash') void purgeClipPermanently(selectedClip.id);
          else void deleteClip(selectedClip.id);
        }
        break;
      case 'toggle-sidebar':
        setIsSidebarCollapsed((collapsed) => !collapsed);
        break;
      case 'zoom-out':
        updateSettings({ textSize: stepAppZoom(textSize, -1) });
        break;
      case 'actual-size':
        updateSettings({ textSize: ACTUAL_SIZE });
        break;
      case 'zoom-in':
        updateSettings({ textSize: stepAppZoom(textSize, 1) });
        break;
      case 'reset-columns':
        resetColumnWidths();
        break;
      case 'refresh-data':
        void refreshData();
        break;
      default:
        break;
    }
  }, enabled);
}
