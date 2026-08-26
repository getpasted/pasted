import type { ComponentProps } from 'react';
import { FilePenLine } from 'lucide-react';

import type { useFeatures } from '../hooks/useFeatures';
import { translate } from '../localization/runtime';
import type { Bin, ClipItem } from '../types';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { ClipBinPicker } from './ClipBinPicker';
import { ClipPreviewNotesPanel } from './ClipPreviewNotesPanel';
import { HotkeyRecorder } from './HotkeyRecorder';
import { OverflowText } from './OverflowText';

type ClipPreviewFeatures = ReturnType<typeof useFeatures>;
type NotesController = ComponentProps<typeof ClipPreviewNotesPanel>['controller'];

export function ClipPreviewOrganization({
  bins,
  clip,
  features,
  notesController,
  onAssignBin,
  onHotkeyChange,
  onName,
  onRemoveBin,
  viewedBinId,
  viewPolicy,
}: {
  bins: Bin[];
  clip: ClipItem;
  features: ClipPreviewFeatures;
  notesController: NotesController;
  onAssignBin: (binId: number | null) => void;
  onHotkeyChange: (hotkey: string | null) => void;
  onName: () => void;
  onRemoveBin: (binId: number) => void;
  viewedBinId?: number | null;
  viewPolicy: ClipViewPolicy;
}) {
  return <>
    {features.bins && viewPolicy.canOrganize && <div className="preview-bin-bar px-4 py-2 flex items-center text-xs border-b">
      <div className="flex min-w-0 items-center">
        <ClipBinPicker
          bins={bins}
          selectedBinIds={clip.bin_ids || []}
          viewedBinId={viewedBinId}
          onClear={() => onAssignBin(null)}
          onToggle={(binId, selected) => {
            if (selected) onAssignBin(binId);
            else onRemoveBin(binId);
          }}
        />
      </div>
      {features.protection && features.hotkeys && <div className="ms-auto flex items-center ps-3">
        <HotkeyRecorder
          value={clip.hotkey ?? null}
          onChange={onHotkeyChange}
        />
      </div>}
    </div>}

    {features.protection && features.hotkeys && !features.bins && viewPolicy.canOrganize && <div className="preview-bin-bar flex items-center justify-end border-b px-4 py-2 text-xs">
      <HotkeyRecorder
        value={clip.hotkey ?? null}
        onChange={onHotkeyChange}
      />
    </div>}

    {features.naming && clip.name && <button
      type="button"
      onClick={onName}
      disabled={!viewPolicy.canOrganize}
      className="preview-name-row theme-named-text flex w-full items-center gap-2.5 border-b px-4 py-3 text-start disabled:cursor-default"
      title={viewPolicy.canOrganize ? translate('action.editName') : clip.name}
      aria-label={viewPolicy.canOrganize ? translate('action.editName') : clip.name}
    >
      <FilePenLine className="h-5 w-5 shrink-0" />
      <OverflowText text={clip.name} className="min-w-0 truncate text-lg font-semibold" />
    </button>}

    {features.notes && <ClipPreviewNotesPanel
      controller={notesController}
      readOnly={!viewPolicy.canEditNotes}
    />}
  </>;
}
