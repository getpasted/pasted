import type { Dispatch, SetStateAction } from 'react';
import type { Bin, ClipItem, SavedTransform } from '../types';
import type { ClipContextMenuState } from '../hooks/useAppOverlays';
import { useLiveClipSnapshot } from '../hooks/useClipViews';
import { getClipViewPolicy } from '../utils/clipViewPolicy';
import { ContextMenu } from './ContextMenu';

interface ClipContextMenuLayerProps {
  menu: ClipContextMenuState | null;
  setMenu: Dispatch<SetStateAction<ClipContextMenuState | null>>;
  clips: ClipItem[];
  trashedClips: ClipItem[];
  currentTab: string;
  selectedClipIds: Set<number>;
  bins: Bin[];
  queuedIndexMap: Map<string, number>;
  trashEnabled: boolean;
  onCopy: (clip: ClipItem) => void;
  onAssignBin: (clipId: number, binId: number | null) => void;
  onRemoveBin: (clipId: number, binId: number) => void;
  onRunTransform: (clip: ClipItem, transform: SavedTransform) => void;
  onOpenTransformations: () => void;
  onName: (clip: ClipItem) => void;
  onClearName: (clipId: number) => void;
  onAddNote: (clip: ClipItem) => void;
  onDeleteNote: (clipId: number) => void;
  onToggleQueue: (clip: ClipItem) => void;
  onTogglePin: (clipId: number) => void;
  onToggleProtected: (clipId: number) => void;
  onToggleConcealed: (clipId: number) => void;
  onDelete: (clipId: number, permanently: boolean) => void;
  onRestore: (clipId: number) => void;
  onPurge: (clipId: number) => void;
}

export function ClipContextMenuLayer(props: ClipContextMenuLayerProps) {
  const clip = useLiveClipSnapshot(props.menu?.clip ?? null, props.clips, props.trashedClips);
  if (!props.menu || !clip) return null;
  return <ContextMenu
    x={props.menu.x}
    y={props.menu.y}
    clip={clip}
    viewPolicy={getClipViewPolicy(props.currentTab, clip)}
    selectedCount={props.selectedClipIds.has(clip.id) ? props.selectedClipIds.size : 1}
    bins={props.bins}
    onClose={() => props.setMenu(null)}
    onCopy={() => props.onCopy(clip)}
    onAssignBin={(binId) => props.onAssignBin(clip.id, binId)}
    onRemoveBin={(binId) => props.onRemoveBin(clip.id, binId)}
    onRunTransform={(transform) => props.onRunTransform(clip, transform)}
    onOpenTransformations={props.onOpenTransformations}
    onName={() => props.onName(clip)}
    onClearName={() => props.onClearName(clip.id)}
    onAddNote={() => props.onAddNote(clip)}
    onDeleteNote={() => props.onDeleteNote(clip.id)}
    isQueued={Boolean(clip.text_content && props.queuedIndexMap.has(clip.text_content))}
    onToggleQueue={() => props.onToggleQueue(clip)}
    onTogglePin={() => props.onTogglePin(clip.id)}
    onToggleProtected={() => props.onToggleProtected(clip.id)}
    onToggleConcealed={() => props.onToggleConcealed(clip.id)}
    onDelete={(event) => props.onDelete(clip.id, Boolean(event?.altKey))}
    onRestore={() => props.onRestore(clip.id)}
    onPurge={() => props.onPurge(clip.id)}
    trashEnabled={props.trashEnabled}
  />;
}
