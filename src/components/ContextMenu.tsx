import React, { useEffect, useState } from 'react';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import type { ClipItem, Bin, SavedTransform } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { binTextColor } from '../utils/binColor';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { safeInvoke as invoke } from '../utils/tauri';
import { selectedClipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { useFeatures } from '../hooks/useFeatures';
import { AnchoredMenu, MenuDivider, MenuItem, MenuSubmenu } from './AnchoredMenu';
import {
  Copy,
  FolderPlus,
  Workflow,
  StickyNote,
  ListPlus,
  Pin,
  PinOff,
  Trash2,
  Trash,
  Sparkles,
  Shield,
  ShieldOff,
  RotateCcw,
} from 'lucide-react';

interface ContextMenuProps {
  x: number;
  y: number;
  clip: ClipItem;
  viewPolicy: ClipViewPolicy;
  selectedCount?: number;
  bins: Bin[];
  onClose: () => void;
  onCopy: () => void;
  onAssignBin: (binId: number | null) => void;
  onRunTransform: (transform: SavedTransform) => void;
  onOpenTransformations: () => void;
  onAddNote: () => void;
  onDeleteNote?: () => void;
  onAddToStack: () => void;
  onTogglePin: () => void;
  onToggleProtected?: () => void;
  onDelete: (e?: React.MouseEvent) => void;
  onRestore?: () => void;
  onPurge?: () => void;
  trashEnabled: boolean;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  x,
  y,
  clip,
  viewPolicy,
  selectedCount,
  bins,
  onClose,
  onCopy,
  onAssignBin,
  onRunTransform,
  onOpenTransformations,
  onAddNote,
  onDeleteNote,
  onAddToStack,
  onTogglePin,
  onToggleProtected,
  onDelete,
  onRestore,
  onPurge,
  trashEnabled,
}) => {
  const features = useFeatures();
  const [activeSubmenu, setActiveSubmenu] = useState<'bins' | 'workflow' | null>(null);
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [isLoadingTransforms, setIsLoadingTransforms] = useState(false);
  const isAltPressed = useAltKeyPressed();

  useEffect(() => {
    if (!features.transformations || !viewPolicy.canRunPipelines || clip.content_type === 'file') return;
    let cancelled = false;
    setIsLoadingTransforms(true);
    invoke<SavedTransform[]>('get_saved_transforms')
      .then((items) => {
        if (!cancelled) setTransforms(Array.isArray(items) ? items : []);
      })
      .catch((error) => console.error('Failed to load Transforms:', error))
      .finally(() => {
        if (!cancelled) setIsLoadingTransforms(false);
      });
    return () => {
      cancelled = true;
    };
  }, [clip.content_type, features.transformations, viewPolicy.canRunPipelines]);

  const setSubmenuOpen = (submenu: 'bins' | 'workflow', open: boolean) => {
    setActiveSubmenu((current) => open ? submenu : current === submenu ? null : current);
  };

  return (
    <AnchoredMenu
      anchor={{ kind: 'point', x, y }}
      ariaLabel="Clip actions"
      className="context-menu w-52"
      onClose={onClose}
    >
      {/* Copy */}
      <button
        onClick={() => {
          onCopy();
          onClose();
        }}
        className="theme-menu-item flex w-full items-center justify-between rounded-md px-3 py-1.5"
      >
        <div className="flex items-center space-x-2.5">
          <Copy className="h-3.5 w-3.5 text-blue-400" />
          <span>{UI_COPY.copy}</span>
        </div>
        <kbd className="theme-text-muted font-mono text-[10px]">↵</kbd>
      </button>

      <MenuDivider />

      {/* Bin */}
      {features.bins && viewPolicy.canAssignBins && (
        <MenuSubmenu
          label="Bin"
          icon={<FolderPlus className="h-3.5 w-3.5 text-amber-400" />}
          open={activeSubmenu === 'bins'}
          onOpenChange={(open) => setSubmenuOpen('bins', open)}
        >
            <MenuItem
              onClick={() => {
                onAssignBin(null);
                onClose();
              }}
              className="px-3 py-1.5"
            >
              No Bin
            </MenuItem>

            {bins.filter((b) => !b.smart_rule).map((b) => (
              <MenuItem
                key={b.id}
                onClick={() => {
                  onAssignBin(b.id);
                  onClose();
                }}
                className="gap-2 px-3 py-1.5"
              >
                <span>{formatEmojiIcon(b.icon)}</span>
                <span className="truncate" style={{ color: binTextColor(b.color) }}>{b.name}</span>
              </MenuItem>
            ))}
        </MenuSubmenu>
      )}

      {/* Workflow Submenu */}
      {features.transformations && viewPolicy.canRunPipelines && clip.content_type !== 'file' && (
        <MenuSubmenu
          label="Workflow"
          icon={<Workflow className="h-3.5 w-3.5 text-cyan-400" />}
          open={activeSubmenu === 'workflow'}
          onOpenChange={(open) => setSubmenuOpen('workflow', open)}
          panelClassName="w-60 max-h-64 overflow-y-auto"
        >
            <div>
              {isLoadingTransforms ? (
                <p className="theme-text-muted px-2.5 py-2 text-[10px]">Loading Transforms…</p>
              ) : transforms.length > 0 ? transforms.map((transform) => {
                const usesIntelligence = transform.plan.steps.some((step) => step.executor.kind === 'semantic');
                return (
                  <MenuItem
                    key={transform.stableRef}
                    onClick={() => {
                      onRunTransform(transform);
                      onClose();
                    }}
                    className="gap-2 px-2.5 py-1.5"
                  >
                    <Workflow className="h-3.5 w-3.5 shrink-0 text-cyan-400" />
                    <span className="min-w-0 flex-1 truncate">{transform.name}</span>
                    {usesIntelligence && <Sparkles className="h-3 w-3 shrink-0 text-violet-400" />}
                  </MenuItem>
                );
              }) : (
                <p className="theme-text-muted px-2.5 py-2 text-[10px]">No saved Transforms yet.</p>
              )}
            </div>
            <MenuDivider />
            <MenuItem
              onClick={() => {
                onOpenTransformations();
                onClose();
              }}
              className="gap-2 px-2.5 py-1.5"
            >
              <Workflow className="h-3.5 w-3.5 shrink-0 text-cyan-400" />
              <span>Manage Transforms…</span>
            </MenuItem>
        </MenuSubmenu>
      )}

      <MenuDivider />

      {/* Add / Edit Note */}
      {features.notes && viewPolicy.canEditNotes && <button
        onClick={() => {
          onAddNote();
          onClose();
        }}
        className="theme-menu-item w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md"
      >
        <StickyNote className="w-3.5 h-3.5 text-amber-400" />
        <span>{clip.note ? 'Edit Note' : 'Add Note'}</span>
      </button>}

      {/* Remove Note */}
      {features.notes && viewPolicy.canEditNotes && clip.note && onDeleteNote && (
        <button
          onClick={() => {
            onDeleteNote();
            onClose();
          }}
          className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-red-500/20 text-red-300 transition-colors"
        >
          <Trash2 className="w-3.5 h-3.5 text-red-400" />
          <span>Remove Note</span>
        </button>
      )}

      {/* Add to Stack */}
      {features.queue && viewPolicy.canOrganize && clip.content_type !== 'file' && Boolean(clip.text_content) && <button
        onClick={() => {
          onAddToStack();
          onClose();
        }}
        className="theme-menu-item w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md"
      >
        <ListPlus className="w-3.5 h-3.5 text-purple-400" />
        <span>Add to Queue</span>
      </button>}

      {/* Toggle Pin */}
      {features.pinning && viewPolicy.canOrganize && <button
        onClick={() => {
          onTogglePin();
          onClose();
        }}
        className="theme-menu-item w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md"
      >
        {clip.is_pinned ? (
          <PinOff className="w-3.5 h-3.5 text-gray-400" />
        ) : (
          <Pin className="w-3.5 h-3.5 pin-icon" />
        )}
        <span>
          {selectedCount && selectedCount > 1
            ? clip.is_pinned
              ? `Unpin ${selectedCount}`
              : `Pin ${selectedCount}`
            : clip.is_pinned
            ? 'Unpin'
            : 'Pin'}
        </span>
      </button>}

      {/* Toggle Protected */}
      {features.protection && viewPolicy.canOrganize && onToggleProtected && (
        <button
          onClick={() => {
            onToggleProtected();
            onClose();
          }}
          className="theme-menu-item w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md"
        >
          {clip.is_protected ? (
            <ShieldOff className="w-3.5 h-3.5 text-gray-400" />
          ) : (
            <Shield className="w-3.5 h-3.5 text-cyan-400" />
          )}
          <span>{clip.is_protected ? 'Unprotect' : 'Protect'}</span>
        </button>
      )}

      <MenuDivider />

      {viewPolicy.state === 'trash' ? (
        <>
          <button
            onClick={() => {
              onRestore?.();
              onClose();
            }}
          className="theme-menu-item flex w-full items-center space-x-2.5 rounded-md px-3 py-1.5"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>{UI_COPY.restore}</span>
          </button>
          <button
            onClick={() => {
              onPurge?.();
              onClose();
            }}
          className="theme-menu-item theme-danger-text flex w-full items-center space-x-2.5 rounded-md px-3 py-1.5"
          >
            <Trash className="w-3.5 h-3.5" />
            <span>{UI_COPY.deletePermanently}</span>
          </button>
        </>
      ) : <button
        onClick={(e) => {
          if (!clip.is_protected) {
            onDelete(e);
            onClose();
          }
        }}
        disabled={clip.is_protected}
        className={`theme-menu-item flex w-full items-center space-x-2.5 rounded-md px-3 py-1.5 ${
          clip.is_protected ? 'cursor-not-allowed opacity-40' : ''
        }`}
      >
        {isAltPressed || !trashEnabled
          ? <Trash className="theme-danger-text w-3.5 h-3.5" />
          : <Trash2 className="theme-danger-text w-3.5 h-3.5" />}
        <span>
          {clip.is_protected
            ? 'Protected'
            : selectedClipDeleteLabel({
                count: selectedCount ?? 1,
                trashEnabled,
                permanent: isAltPressed,
              })}
        </span>
      </button>}
    </AnchoredMenu>
  );
};
