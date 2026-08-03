import React, { useEffect, useRef, useState } from 'react';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { ClipItem, Bin, FilterRule } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { detectSmartFilterRecommendations } from '../utils/smartFilterDetector';
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
} from 'lucide-react';

interface ContextMenuProps {
  x: number;
  y: number;
  clip: ClipItem;
  selectedCount?: number;
  bins: Bin[];
  filters: FilterRule[];
  onClose: () => void;
  onCopy: () => void;
  onAssignBin: (binId: number | null) => void;
  onApplyFilter: (filter: FilterRule) => void;
  onAddNote: () => void;
  onDeleteNote?: () => void;
  onAddToStack: () => void;
  onTogglePin: () => void;
  onToggleProtected?: () => void;
  onDelete: (e?: React.MouseEvent) => void;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  x,
  y,
  clip,
  selectedCount,
  bins,
  filters,
  onClose,
  onCopy,
  onAssignBin,
  onApplyFilter,
  onAddNote,
  onDeleteNote,
  onAddToStack,
  onTogglePin,
  onToggleProtected,
  onDelete,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubmenu, setActiveSubmenu] = useState<'bins' | 'filters' | null>(null);
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
      className="fixed z-50 w-52 glass-hud rounded-xl py-1.5 px-1 shadow-2xl border border-gray-700/80 text-xs font-medium text-gray-200 select-none animate-in fade-in zoom-in-95 duration-100"
    >
      {/* Copy */}
      <button
        onClick={() => {
          onCopy();
          onClose();
        }}
        className="w-full flex items-center justify-between px-3 py-1.5 rounded-md hover:bg-blue-600 hover:text-white transition-all duration-100 group"
      >
        <div className="flex items-center space-x-2.5">
          <Copy className="w-3.5 h-3.5 text-blue-400 group-hover:text-white transition-colors" />
          <span>Copy</span>
        </div>
        <kbd className="text-[10px] text-gray-400 group-hover:text-gray-200 font-mono">↵</kbd>
      </button>

      <div className="my-1 border-t border-gray-800" />

      {/* Bin */}
      <div
        className="relative"
        onMouseEnter={() => setActiveSubmenu('bins')}
        onMouseLeave={() => setActiveSubmenu(null)}
      >
        <button className="w-full flex items-center justify-between px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white transition-colors">
          <div className="flex items-center space-x-2.5">
            <FolderPlus className="w-3.5 h-3.5 text-amber-400" />
            <span>Bin</span>
          </div>
          <ChevronRight className="w-3.5 h-3.5 text-gray-400" />
        </button>

        {activeSubmenu === 'bins' && (
          <div className="absolute left-full top-0 ml-1 w-48 glass-hud rounded-xl py-1 px-1 shadow-2xl border border-gray-700/80">
            <button
              onClick={() => {
                onAssignBin(null);
                onClose();
              }}
              className="w-full text-left px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white"
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
                className="w-full text-left px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white truncate flex items-center space-x-2"
              >
                <span>{formatEmojiIcon(b.icon)}</span>
                <span className="truncate">{b.name}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Filter Submenu */}
      <div
        className="relative"
        onMouseEnter={() => setActiveSubmenu('filters')}
        onMouseLeave={() => setActiveSubmenu(null)}
      >
        <button className="w-full flex items-center justify-between px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white transition-colors">
          <div className="flex items-center space-x-2.5">
            <Sliders className="w-3.5 h-3.5 text-cyan-400" />
            <span>Apply Filter...</span>
          </div>
          {(() => {
            const { detectedTypes } = detectSmartFilterRecommendations(clip.text_content || '', filters);
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

        {activeSubmenu === 'filters' && (
          <div className="absolute left-full top-0 ml-1 w-56 glass-hud rounded-xl py-1 px-1 shadow-2xl border border-gray-700/80 max-h-64 overflow-y-auto space-y-1">
            {(() => {
              const { recommendedFilterIds, detectedTypes } = detectSmartFilterRecommendations(clip.text_content || '', filters);
              const recommended = filters.filter((f) => recommendedFilterIds.has(f.id));
              const otherFilters = filters.filter((f) => !recommendedFilterIds.has(f.id));

              return (
                <>
                  {recommended.length > 0 && (
                    <div className="pb-1 border-b border-gray-800/80">
                      <div className="px-2 py-1 text-[9px] font-semibold text-cyan-400 uppercase tracking-wider flex items-center space-x-1">
                        <Sparkles className="w-3 h-3 text-cyan-400" />
                        <span>Recommended for {detectedTypes.join(', ')}</span>
                      </div>
                      {recommended.map((f) => (
                        <button
                          key={f.id}
                          onClick={() => {
                            onApplyFilter(f);
                            onClose();
                          }}
                          className="w-full text-left px-2.5 py-1.5 rounded-md hover:bg-cyan-600/30 hover:text-white flex items-center justify-between text-xs font-medium text-cyan-200"
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
                        onApplyFilter(f);
                        onClose();
                      }}
                      className="w-full text-left px-2.5 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white text-xs truncate text-gray-300"
                    >
                      {f.name}
                    </button>
                  ))}
                </>
              );
            })()}
          </div>
        )}
      </div>

      <div className="my-1 border-t border-gray-800" />

      {/* Add / Edit Note */}
      <button
        onClick={() => {
          onAddNote();
          onClose();
        }}
        className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white transition-colors"
      >
        <StickyNote className="w-3.5 h-3.5 text-amber-400" />
        <span>{clip.note ? 'Edit Note' : 'Add Note'}</span>
      </button>

      {/* Remove Note */}
      {clip.note && onDeleteNote && (
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
      <button
        onClick={() => {
          onAddToStack();
          onClose();
        }}
        className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white transition-colors"
      >
        <ListPlus className="w-3.5 h-3.5 text-purple-400" />
        <span>Add to Queue</span>
      </button>

      {/* Toggle Pin */}
      <button
        onClick={() => {
          onTogglePin();
          onClose();
        }}
        className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white transition-colors"
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
      </button>

      {/* Toggle Protected */}
      {onToggleProtected && (
        <button
          onClick={() => {
            onToggleProtected();
            onClose();
          }}
          className="w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md hover:bg-blue-600/30 hover:text-white transition-colors"
        >
          {clip.is_protected ? (
            <ShieldOff className="w-3.5 h-3.5 text-gray-400" />
          ) : (
            <Shield className="w-3.5 h-3.5 text-cyan-400" />
          )}
          <span>{clip.is_protected ? 'Unprotect' : 'Protect'}</span>
        </button>
      )}

      <div className="my-1 border-t border-gray-800" />

      {/* Delete */}
      <button
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
      </button>
    </div>
  );
};
