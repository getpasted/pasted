import React, { useEffect, useRef, useState } from 'react';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { ClipItem, Bin, Pipeline } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { detectSmartPipelineRecommendations } from '../utils/smartPipelineDetector';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import {
  Copy,
  FolderPlus,
  Sliders,
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
  ClipboardPaste,
} from 'lucide-react';

export type PipelineDestination = 'copy' | 'paste';

interface ContextMenuProps {
  x: number;
  y: number;
  clip: ClipItem;
  viewPolicy: ClipViewPolicy;
  selectedCount?: number;
  bins: Bin[];
  pipelines: Pipeline[];
  onClose: () => void;
  onCopy: () => void;
  onAssignBin: (binId: number | null) => void;
  onRunPipeline: (pipeline: Pipeline, destination: PipelineDestination) => void;
  onAddNote: () => void;
  onDeleteNote?: () => void;
  onAddToStack: () => void;
  onTogglePin: () => void;
  onToggleProtected?: () => void;
  onDelete: (e?: React.MouseEvent) => void;
  onRestore?: () => void;
  onPurge?: () => void;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  x,
  y,
  clip,
  viewPolicy,
  selectedCount,
  bins,
  pipelines,
  onClose,
  onCopy,
  onAssignBin,
  onRunPipeline,
  onAddNote,
  onDeleteNote,
  onAddToStack,
  onTogglePin,
  onToggleProtected,
  onDelete,
  onRestore,
  onPurge,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubmenu, setActiveSubmenu] = useState<'bins' | 'pipelines' | null>(null);
  const [pipelineDestination, setPipelineDestination] = useState<PipelineDestination>('copy');
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
    >
      {/* Copy */}
      <button
        onClick={() => {
          onCopy();
          onClose();
        }}
        className="theme-menu-item w-full flex items-center justify-between px-3 py-1.5 rounded-md group"
      >
        <div className="flex items-center space-x-2.5">
          <Copy className="w-3.5 h-3.5 text-blue-400 group-hover:text-white transition-colors" />
          <span>Copy</span>
        </div>
        <kbd className="text-[10px] text-gray-400 group-hover:text-gray-200 font-mono">↵</kbd>
      </button>

      <div className="theme-menu-divider my-1 border-t" />

      {/* Bin */}
      {viewPolicy.canAssignBins && <div
        className="relative"
        onMouseEnter={() => setActiveSubmenu('bins')}
        onMouseLeave={() => setActiveSubmenu(null)}
      >
        <button className="theme-menu-item w-full flex items-center justify-between px-3 py-1.5 rounded-md">
          <div className="flex items-center space-x-2.5">
            <FolderPlus className="w-3.5 h-3.5 text-amber-400" />
            <span>Bin</span>
          </div>
          <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
        </button>

        {activeSubmenu === 'bins' && (
          <div className="theme-menu absolute left-full top-0 ml-1 w-48 rounded-xl py-1 px-1 border">
            <button
              onClick={() => {
                onAssignBin(null);
                onClose();
              }}
              className="theme-menu-item w-full text-left px-3 py-1.5 rounded-md"
            >
              – None –
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
                <span className="truncate">{b.name}</span>
              </button>
            ))}
          </div>
        )}
      </div>}

      {/* Filter Submenu */}
      {viewPolicy.canRunPipelines && <div
        className="relative"
        onMouseEnter={() => setActiveSubmenu('pipelines')}
        onMouseLeave={() => setActiveSubmenu(null)}
      >
        <button
          type="button"
          onClick={() => setActiveSubmenu('pipelines')}
          onFocus={() => setActiveSubmenu('pipelines')}
          aria-haspopup="menu"
          aria-expanded={activeSubmenu === 'pipelines'}
          className="theme-menu-item w-full flex items-center justify-between px-3 py-1.5 rounded-md"
        >
          <div className="flex items-center space-x-2.5">
            <Sliders className="w-3.5 h-3.5 text-cyan-400" />
            <span>Apply Pipeline...</span>
          </div>
          {(() => {
            const { detectedTypes } = detectSmartPipelineRecommendations(clip.text_content || '', pipelines);
            return detectedTypes.length > 0 ? (
              <span className="text-[9px] font-mono text-cyan-400 bg-cyan-950/60 border border-cyan-800/60 px-1 py-0.2 rounded flex items-center space-x-0.5">
                <Sparkles className="w-2.5 h-2.5" />
                <span>{detectedTypes[0]}</span>
              </span>
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
            );
          })()}
        </button>

        {activeSubmenu === 'pipelines' && (
          <div className="theme-menu absolute left-full top-0 ml-1 w-56 rounded-xl py-1 px-1 border max-h-64 overflow-y-auto space-y-1">
            <div className="theme-subtle-surface m-1 grid grid-cols-2 gap-1 rounded-lg border p-1">
              <button
                type="button"
                onClick={() => setPipelineDestination('copy')}
                className={`theme-menu-item flex items-center justify-center gap-1.5 rounded-md px-2 py-1 text-[10px] font-semibold ${pipelineDestination === 'copy' ? 'is-selected' : ''}`}
              >
                <Copy className="h-3 w-3" />
                <span>Copy Result</span>
              </button>
              <button
                type="button"
                onClick={() => setPipelineDestination('paste')}
                className={`theme-menu-item flex items-center justify-center gap-1.5 rounded-md px-2 py-1 text-[10px] font-semibold ${pipelineDestination === 'paste' ? 'is-selected' : ''}`}
              >
                <ClipboardPaste className="h-3 w-3" />
                <span>Paste Result</span>
              </button>
            </div>
            {(() => {
              const { recommendedPipelineIds, detectedTypes } = detectSmartPipelineRecommendations(clip.text_content || '', pipelines);
              const recommended = pipelines.filter((pipeline) => recommendedPipelineIds.has(pipeline.id));
              const otherFilters = pipelines.filter((pipeline) => !recommendedPipelineIds.has(pipeline.id));

              return (
                <>
                  {recommended.length > 0 && (
                    <div className="theme-menu-divider pb-1 border-b">
                      <div className="px-2 py-1 text-[9px] font-semibold text-cyan-400 uppercase tracking-wider flex items-center space-x-1">
                        <Sparkles className="w-3 h-3 text-cyan-400" />
                        <span>Recommended for {detectedTypes.join(', ')}</span>
                      </div>
                      {recommended.map((f) => (
                        <button
                          key={f.id}
                          onClick={() => {
                            onRunPipeline(f, pipelineDestination);
                            onClose();
                          }}
                          className="theme-menu-item smart-menu-item w-full text-left px-2.5 py-1.5 rounded-md flex items-center justify-between text-xs font-medium"
                        >
                          <span className="truncate">{f.name}</span>
                          <Sparkles className="w-3 h-3 text-cyan-400 shrink-0 ml-1" />
                        </button>
                      ))}
                    </div>
                  )}

                  {otherFilters.map((f) => (
                    <button
                      key={f.id}
                      onClick={() => {
                        onRunPipeline(f, pipelineDestination);
                        onClose();
                      }}
                      className="theme-menu-item w-full text-left px-2.5 py-1.5 rounded-md text-xs truncate"
                    >
                      {f.name}
                    </button>
                  ))}
                </>
              );
            })()}
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
              ? `Unpin ${selectedCount} Items`
              : `Pin ${selectedCount} Items`
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
            className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-cyan-500/20 text-cyan-300 transition-colors"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Restore from Trash</span>
          </button>
          <button
            onClick={() => {
              onPurge?.();
              onClose();
            }}
            className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-red-500/20 text-red-400 hover:text-red-300 transition-colors"
          >
            <Trash className="w-3.5 h-3.5" />
            <span>Delete Permanently</span>
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
        className={`w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md transition-colors ${
          clip.is_protected
            ? 'opacity-40 cursor-not-allowed text-gray-500'
            : 'hover:bg-red-500/20 text-red-400 hover:text-red-300'
        }`}
      >
        {isAltPressed ? <Trash className="w-3.5 h-3.5" /> : <Trash2 className="w-3.5 h-3.5" />}
        <span>
          {clip.is_protected
            ? 'Item is Protected'
            : selectedCount && selectedCount > 1
            ? `Move ${selectedCount} Items to Trash`
            : isAltPressed
            ? 'Delete Permanently (Option held)'
            : 'Move to Trash'}
        </span>
      </button>}
    </div>
  );
};
