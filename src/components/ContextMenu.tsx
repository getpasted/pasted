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
import { OverflowText } from './OverflowText';
import { translate } from '../localization/runtime';
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
  Check,
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
  onRemoveBin: (binId: number) => void;
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
  onRemoveBin,
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
  const isExplicitlyProtected = clip.is_explicitly_protected ?? clip.is_protected ?? false;
  const inheritedProtectionOnly = Boolean(clip.is_protected) && !isExplicitlyProtected;
  const protectionToggleDisabled = Boolean(clip.shortcut) || inheritedProtectionOnly;

  useEffect(() => {
    if (!features.transformations || !viewPolicy.canRunPipelines || clip.content_type === 'file') return;
    let cancelled = false;
    setIsLoadingTransforms(true);
    invoke<SavedTransform[]>('get_intent_transforms')
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
      ariaLabel={translate('component.contextMenu.clipActions')}
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
          <Copy className="theme-status-info-text h-3.5 w-3.5" />
          <span>{UI_COPY.copy}</span>
        </div>
        <kbd className="theme-text-muted font-mono text-[10px]">↵</kbd>
      </button>

      <MenuDivider />

      {/* Bin */}
      {features.bins && viewPolicy.canAssignBins && (
        <MenuSubmenu
          label={translate('component.contextMenu.bin')}
          icon={<FolderPlus className="theme-status-warning-text h-3.5 w-3.5" />}
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
              {translate('common.noBin')}
            </MenuItem>

            {bins.filter((b) => !b.smart_rule).map((b) => {
              const active = Boolean(clip.bin_ids?.includes(b.id));
              return (
                <MenuItem
                  key={b.id}
                  onClick={() => {
                    if (active) onRemoveBin(b.id);
                    else onAssignBin(b.id);
                  }}
                  className="gap-2 px-3 py-1.5"
                  role="menuitemcheckbox"
                  aria-checked={active}
                  active={active}
                >
                  <span>{formatEmojiIcon(b.icon)}</span>
                  <OverflowText text={b.name} className="bidi-interface-align truncate" style={{ color: binTextColor(b.color) }} />
                  {active && <Check className="ms-auto h-3.5 w-3.5" aria-hidden="true" />}
                </MenuItem>
              );
            })}
            {bins.some((b) => b.smart_rule) && (
              <>
                <MenuDivider />
                <div className="theme-text-subtle px-3 pb-1 pt-1 text-[10px] font-bold uppercase tracking-wider">
                  {translate('component.contextMenu.smartBinsAutomatic')}
                </div>
              </>
            )}
            {bins.filter((b) => b.smart_rule).map((b) => (
              <MenuItem
                key={b.id}
                disabled
                role="menuitemcheckbox"
                aria-checked={Boolean(clip.bin_ids?.includes(b.id))}
                active={Boolean(clip.bin_ids?.includes(b.id))}
                title={translate('component.contextMenu.smartBinMembershipIsManagedAutomatically')}
                className="gap-2 px-3 py-1.5"
              >
                <span>{formatEmojiIcon(b.icon)}</span>
                <OverflowText text={b.name} className="bidi-interface-align truncate" style={{ color: binTextColor(b.color) }} />
                {clip.bin_ids?.includes(b.id) && <Check className="ms-auto h-3.5 w-3.5" aria-hidden="true" />}
              </MenuItem>
            ))}
        </MenuSubmenu>
      )}

      {/* Workflow Submenu */}
      {features.transformations && viewPolicy.canRunPipelines && clip.content_type !== 'file' && (
        <MenuSubmenu
          label={translate('component.contextMenu.workflow')}
          icon={<Workflow className="theme-workflow-text h-3.5 w-3.5" />}
          open={activeSubmenu === 'workflow'}
          onOpenChange={(open) => setSubmenuOpen('workflow', open)}
          panelClassName="w-60 max-h-64 overflow-y-auto"
        >
            <div>
              {isLoadingTransforms ? (
                <p className="theme-text-muted px-2.5 py-2 text-[10px]">{translate('component.contextMenu.loadingTransforms')}</p>
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
                    <Workflow className="theme-workflow-text h-3.5 w-3.5 shrink-0" />
                    <OverflowText text={transform.name} className="bidi-interface-align min-w-0 flex-1 truncate" />
                    {usesIntelligence && <Sparkles className="theme-intelligence-text h-3 w-3 shrink-0" />}
                  </MenuItem>
                );
              }) : (
                <p className="theme-text-muted px-2.5 py-2 text-[10px]">{translate('component.contextMenu.noSavedTransformsYet')}</p>
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
              <Workflow className="theme-workflow-text h-3.5 w-3.5 shrink-0" />
              <span>{translate('component.contextMenu.manageTransforms')}</span>
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
        <StickyNote className="theme-note-text w-3.5 h-3.5" />
        <span>{clip.note ? translate('action.editNote') : translate('action.addNote')}</span>
      </button>}

      {/* Remove Note */}
      {features.notes && viewPolicy.canEditNotes && clip.note && onDeleteNote && (
        <button
          onClick={() => {
            onDeleteNote();
            onClose();
          }}
          className="theme-menu-item theme-danger-text w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md transition-colors"
        >
          <Trash2 className="w-3.5 h-3.5" />
          <span>{translate('component.contextMenu.removeNote')}</span>
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
        <ListPlus className="theme-queue-text w-3.5 h-3.5" />
        <span>{translate('component.contextMenu.addToQueue')}</span>
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
          <PinOff className="theme-text-muted w-3.5 h-3.5" />
        ) : (
          <Pin className="w-3.5 h-3.5 pin-icon" />
        )}
        <span>
          {selectedCount && selectedCount > 1
            ? clip.is_pinned
              ? translate('component.contextMenu.unpinCount', { count: selectedCount })
              : translate('component.contextMenu.pinCount', { count: selectedCount })
            : clip.is_pinned
            ? translate('action.unpin')
            : translate('action.pin')}
        </span>
      </button>}

      {/* Toggle Protected */}
      {features.protection && viewPolicy.canOrganize && onToggleProtected && (
        <button
          onClick={() => {
            onToggleProtected();
            onClose();
          }}
          disabled={protectionToggleDisabled}
          title={clip.shortcut
            ? translate('component.clipPreview.removeShortcutBeforeUnprotecting')
            : inheritedProtectionOnly
              ? translate('component.clipPreview.protectedByBin')
              : undefined}
          className="theme-menu-item w-full flex items-center space-x-2.5 px-3 py-1.5 rounded-md"
        >
          {clip.is_protected && !protectionToggleDisabled ? (
            <ShieldOff className="theme-text-muted w-3.5 h-3.5" />
          ) : (
            <Shield className="theme-status-info-text w-3.5 h-3.5" />
          )}
          <span>{protectionToggleDisabled
            ? translate('component.contextMenu.protectedAutomatically')
            : clip.is_protected ? translate('action.unprotect') : translate('action.protect')}</span>
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
            ? translate('component.contextMenu.protected')
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
