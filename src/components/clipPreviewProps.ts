import type { AppSettings, Bin, ClipItem, ManualTransform } from '../types';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';

export interface ClipPreviewProps {
  clip: ClipItem | null;
  viewPolicy: ClipViewPolicy;
  bins: Bin[];
  viewedBinId?: number | null;
  manualTransforms: ManualTransform[];
  onUpdateClip: (updatedClip?: ClipItem) => void;
  onAssignBin: (clipId: number, binId: number | null) => void | Promise<void>;
  onRemoveBin: (clipId: number, binId: number) => void | Promise<void>;
  onTogglePin: (clipId: number) => void;
  onToggleProtected: (clipId: number) => void;
  onToggleConcealed: (clipId: number) => void;
  onName: (clip: ClipItem) => void;
  onDeleteClip: (id: number) => void;
  onRestoreClip: (id: number) => void;
  onUpdateClipNote?: (clipId: number, noteContent: string | null) => void;
  isTransforming?: boolean;
  transformError?: string;
  onOpenTransformations?: () => void;
  onOpenIntelligence?: () => void;
  trashEnabled: boolean;
  filePreviewMode: AppSettings['filePreviewMode'];
  filePreviewMaxMb: number;
}
