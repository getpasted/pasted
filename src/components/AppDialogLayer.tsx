import type { Dispatch, SetStateAction } from 'react';
import type { AppSettings, Bin, ClipCollectionSummary, ClipItem } from '../types';
import type { FeatureId } from '../utils/features';
import type { BinContextMenuState } from '../hooks/useAppOverlays';
import type { ClearHistoryMode } from './ClearHistoryDialog';
import { binsApi } from '../api/bins';
import { clipsApi } from '../api/clips';
import { BinContextMenu } from './BinContextMenu';
import { BinModal } from './BinModal';
import { ClearHistoryDialog } from './ClearHistoryDialog';
import { ClipNoteDialog } from './ClipNoteDialog';
import { DeleteBinDialog } from './DeleteBinDialog';
import { WelcomeSetup } from './WelcomeSetup';

interface AppDialogLayerProps {
  features: Record<FeatureId, boolean>;
  settings: AppSettings;
  settingsHydrated: boolean;
  initialDataLoaded: boolean;
  updateSettings: (updates: Partial<AppSettings>) => void;
  bins: Bin[];
  clipCollectionSummary: ClipCollectionSummary;
  selectedBinId: number | null;
  setSelectedBinId: Dispatch<SetStateAction<number | null>>;
  navigateToTab: (route: string) => void;
  binContextMenu: BinContextMenuState | null;
  setBinContextMenu: Dispatch<SetStateAction<BinContextMenuState | null>>;
  isBinModalOpen: boolean;
  editingBin: Bin | null;
  editBin: (bin: Bin) => void;
  closeBinModal: () => void;
  binToDelete: Bin | null;
  setBinToDelete: Dispatch<SetStateAction<Bin | null>>;
  notePromptClip: ClipItem | null;
  setNotePromptClip: Dispatch<SetStateAction<ClipItem | null>>;
  notePromptText: string;
  setNotePromptText: Dispatch<SetStateAction<string>>;
  updateClipNoteLocally: (clipId: number, note: string | null) => void;
  clearHistoryMode: ClearHistoryMode | null;
  setClearHistoryMode: Dispatch<SetStateAction<ClearHistoryMode | null>>;
  confirmClearHistory: () => Promise<void>;
  fetchBins: () => Promise<void>;
  fetchClips: () => Promise<void>;
  fetchTrashedClips: () => Promise<void>;
  fetchClipCollectionSummary: () => Promise<void>;
}

export function AppDialogLayer({
  features,
  settings,
  settingsHydrated,
  initialDataLoaded,
  updateSettings,
  bins,
  clipCollectionSummary,
  selectedBinId,
  setSelectedBinId,
  navigateToTab,
  binContextMenu,
  setBinContextMenu,
  isBinModalOpen,
  editingBin,
  editBin,
  closeBinModal,
  binToDelete,
  setBinToDelete,
  notePromptClip,
  setNotePromptClip,
  notePromptText,
  setNotePromptText,
  updateClipNoteLocally,
  clearHistoryMode,
  setClearHistoryMode,
  confirmClearHistory,
  fetchBins,
  fetchClips,
  fetchTrashedClips,
  fetchClipCollectionSummary,
}: AppDialogLayerProps) {
  return <>
    {features.bins && binContextMenu && (
      <BinContextMenu
        menu={binContextMenu}
        onClose={() => setBinContextMenu(null)}
        onEdit={(bin) => {
          setBinContextMenu(null);
          editBin(bin);
        }}
        onDelete={(bin) => {
          setBinContextMenu(null);
          setBinToDelete(bin);
        }}
      />
    )}

    {features.bins && (
      <BinModal
        key={editingBin ? `edit-${editingBin.id}` : 'new-bin'}
        isOpen={isBinModalOpen}
        editingBin={editingBin}
        features={{
          clipTypes: features.clipTypes,
          fileFormats: features.fileFormats,
          sources: features.sources,
          protection: features.protection,
          concealment: features.concealment,
          types: features.types,
        }}
        fileFormats={clipCollectionSummary.fileFormatCounts.map(({ file_format }) => file_format)}
        sources={clipCollectionSummary.sourceCounts.map(({ name }) => name)}
        onClose={closeBinModal}
        onRefreshBins={fetchBins}
      />
    )}

    {features.bins && binToDelete && (
      <DeleteBinDialog
        bin={binToDelete}
        bins={bins}
        onCancel={() => setBinToDelete(null)}
        onConfirm={async (bin, disposition, destinationBinId) => {
          try {
            await binsApi.delete(bin.id, disposition, destinationBinId);
            setBinToDelete(null);
            await Promise.all([fetchBins(), fetchClips(), fetchTrashedClips()]);
            if (selectedBinId === bin.id) {
              navigateToTab('all');
              setSelectedBinId(null);
            }
          } catch (error) {
            console.error(error);
          }
        }}
      />
    )}

    {features.notes && notePromptClip && (
      <ClipNoteDialog
        clip={notePromptClip}
        text={notePromptText}
        onTextChange={setNotePromptText}
        onCancel={() => setNotePromptClip(null)}
        onSave={async (clip, note) => {
          updateClipNoteLocally(clip.id, note);
          setNotePromptClip(null);
          try {
            await clipsApi.updateNote(clip.id, note);
            await fetchClipCollectionSummary();
          } catch (error) {
            console.error(error);
            void fetchClips();
          }
        }}
      />
    )}

    {clearHistoryMode && (
      <ClearHistoryDialog
        mode={clearHistoryMode}
        onCancel={() => setClearHistoryMode(null)}
        onConfirm={confirmClearHistory}
      />
    )}

    <WelcomeSetup
      isOpen={settingsHydrated && initialDataLoaded && settings.onboardingVersion < 1}
      settings={settings}
      onUpdateSettings={updateSettings}
      onImported={async () => {
        await Promise.all([fetchClips(), fetchTrashedClips(), fetchBins()]);
      }}
    />
  </>;
}
