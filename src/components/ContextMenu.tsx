import React, { useEffect, useRef, useState } from 'react';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import type { ClipItem, Bin, SavedTransform } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { binTextColor } from '../utils/binColor';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { safeInvoke as invoke } from '../utils/tauri';
import { selectedClipDeleteLabel, UI_COPY } from '../utils/uiCopy';
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
  ChevronRight,
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
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubmenu, setActiveSubmenu] = useState<'bins' | 'workflow' | null>(null);
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [isLoadingTransforms, setIsLoadingTransforms] = useState(false);
  const isAltPressed = useAltKeyPressed();

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    window.addEventListener('mousedown', handleClickOutside);
    return () => window.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  useEffect(() => {
    if (!viewPolicy.canRunPipelines || clip.content_type === 'file') return;
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
  }, [clip.content_type, viewPolicy.canRunPipelines]);

  // Adjust coordinates if menu goes off screen
  const menuWidth = 220;
  const menuHeight = 320;
  const adjustedX = Math.min(x, window.innerWidth - menuWidth - 10);
  const adjustedY = Math.min(y, window.innerHeight - menuHeight - 10);

  return (
    <div
      ref={menuRef}
      style={{ left: `${adjustedX}px`, top: `${adjustedY}px` }}
      className="theme-menu context-menu fixed w-52 rounded-xl py-1.5 px-1 border text-xs font-medium select-none animate-in fade-in zoom-in-95 duration-100"
      role="menu"
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

      <div className="theme-menu-divider my-1 border-t" />

      {/* Bin */}
      {viewPolicy.canAssignBins && <div
        className="relative"
        onMouseEnter={() => setActiveSubmenu('bins')}
        onMouseLeave={() => setActiveSubmenu(null)}
      >
        <button
          type="button"
          aria-haspopup="menu"
          aria-expanded={activeSubmenu === 'bins'}
          className={`theme-menu-item w-full flex items-center justify-between px-3 py-1.5 rounded-md ${activeSubmenu === 'bins' ? 'is-selected' : ''}`}
        >
          <div className="flex items-center space-x-2.5">
            <FolderPlus className="w-3.5 h-3.5 text-amber-400" />
            <span>Bin</span>
          </div>
          <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
        </button>

        {activeSubmenu === 'bins' && (
          <div className="theme-menu absolute left-[calc(100%-1px)] -top-1 w-48 rounded-xl py-1 px-1 border" role="menu">
            <button
              onClick={() => {
                onAssignBin(null);
                onClose();
              }}
              className="theme-menu-item w-full text-left px-3 py-1.5 rounded-md"
            >
              No Bin
            </button>

            {bins.filter((b) => !b.smart_rule).map((b) => (
              <button
                key={b.id}
                onClick={() => {
                  onAssignBin(b.id);
                  onClose();
                }}
                className="theme-menu-item w-full text-left px-3 py-1.5 rounded-md truncate flex items-center space-x-2"
              >
                <span>{formatEmojiIcon(b.icon)}</span>
                <span className="truncate" style={{ color: binTextColor(b.color) }}>{b.name}</span>
              </button>
            ))}
          </div>
        )}
      </div>}

      {/* Workflow Submenu */}
      {viewPolicy.canRunPipelines && clip.content_type !== 'file' && <div
        className="relative"
        onMouseEnter={() => setActiveSubmenu('workflow')}
        onMouseLeave={() => setActiveSubmenu(null)}
      >
        <button
          type="button"
          onClick={() => setActiveSubmenu('workflow')}
          onFocus={() => setActiveSubmenu('workflow')}
          aria-haspopup="menu"
          aria-expanded={activeSubmenu === 'workflow'}
          className={`theme-menu-item w-full flex items-center justify-between px-3 py-1.5 rounded-md ${activeSubmenu === 'workflow' ? 'is-selected' : ''}`}
        >
          <div className="flex items-center space-x-2.5">
            <Workflow className="w-3.5 h-3.5 text-cyan-400" />
            <span>Workflow</span>
          </div>
          <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
        </button>

        {activeSubmenu === 'workflow' && (
          <div className="theme-menu absolute left-[calc(100%-1px)] -top-1 w-60 rounded-xl border p-1 max-h-64 overflow-y-auto" role="menu">
            <div>
              {isLoadingTransforms ? (
                <p className="theme-text-muted px-2.5 py-2 text-[10px]">Loading Transforms…</p>
              ) : transforms.length > 0 ? transforms.map((transform) => {
                const usesIntelligence = transform.plan.steps.some((step) => step.executor.kind === 'semantic');
                return (
                  <button
                    key={transform.stableRef}
                    type="button"
                    onClick={() => {
                      onRunTransform(transform);
                      onClose();
                    }}
                    className="theme-menu-item flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left"
                  >
                    <Workflow className="h-3.5 w-3.5 shrink-0 text-cyan-400" />
                    <span className="min-w-0 flex-1 truncate">{transform.name}</span>
                    {usesIntelligence && <Sparkles className="h-3 w-3 shrink-0 text-violet-400" />}
                  </button>
                );
              }) : (
                <p className="theme-text-muted px-2.5 py-2 text-[10px]">No saved Transforms yet.</p>
              )}
            </div>
            <div className="theme-menu-divider my-1 border-t" />
            <button
              type="button"
              onClick={() => {
                onOpenTransformations();
                onClose();
              }}
              className="theme-menu-item flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left"
            >
              <Workflow className="h-3.5 w-3.5 shrink-0 text-cyan-400" />
              <span>Manage Transforms…</span>
            </button>
          </div>
        )}
      </div>}

      <div className="theme-menu-divider my-1 border-t" />

      {/* Add / Edit Note */}
      {viewPolicy.canEditNotes && <button
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
      {viewPolicy.canEditNotes && clip.note && onDeleteNote && (
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
      {viewPolicy.canOrganize && <button
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
      {viewPolicy.canOrganize && <button
        onClick={() => {
          onTogglePin();
          onClose();
        }}
        className="theme-menu-item w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md"
      >
        {clip.is_pinned ? (
          <PinOff className="w-3.5 h-3.5 text-gray-400" />
        ) : (
          <Pin className="w-3.5 h-3.5 text-orange-500 fill-orange-500/20 pin-icon" />
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
      {viewPolicy.canOrganize && onToggleProtected && (
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

      <div className="theme-menu-divider my-1 border-t" />

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
          clip.is_protected
            ? 'cursor-not-allowed opacity-40'
            : 'theme-danger-text'
        }`}
      >
        {isAltPressed || !trashEnabled ? <Trash className="w-3.5 h-3.5" /> : <Trash2 className="w-3.5 h-3.5" />}
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
    </div>
  );
};
