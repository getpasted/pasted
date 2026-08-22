import type { RefObject } from 'react';
import {
  Check,
  Copy,
  Eye,
  EyeOff,
  FilePenLine,
  LoaderCircle,
  Pin,
  Shield,
  ShieldOff,
  Sparkles,
  StickyNote,
  Trash2,
  Workflow,
  X,
} from 'lucide-react';

import type { useFeatures } from '../hooks/useFeatures';
import { translate } from '../localization/runtime';
import { localizedSourceName } from '../localization/presentation';
import { getClipFilePaths, type ClipItem, type ClipTransformationProvenance, type SavedTransform } from '../types';
import { contentTypeLabel, structuralClipType } from '../utils/contentTypes';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import type { ClipContentMatch } from './clipPreviewModel';
import { contentMatchTitle } from './clipPreviewModel';
import { ClipWorkflowMenu } from './ClipWorkflowMenu';
import { OverflowText } from './OverflowText';

type ClipPreviewFeatures = ReturnType<typeof useFeatures>;

export function ClipPreviewHeader({
  activeTransformRef,
  canTransformContent,
  clip,
  concealmentEffective,
  contentMatches,
  copied,
  features,
  hiddenContentTypes,
  isAddingNote,
  isManualTransformRunning,
  isTransforming,
  isWorkflowMenuOpen,
  onCopy,
  onCloseWorkflowMenu,
  onDelete,
  onManageTransforms,
  onName,
  onPreviewTransform,
  onToggleConcealed,
  onToggleNote,
  onTogglePin,
  onToggleProtected,
  onToggleWorkflowMenu,
  protectedByBin,
  protectionToggleDisabled,
  provenance,
  transforms,
  trashEnabled,
  viewPolicy,
  visibleContentTypes,
  workflowTriggerRef,
}: {
  activeTransformRef: string | null;
  canTransformContent: boolean;
  clip: ClipItem;
  concealmentEffective: boolean;
  contentMatches: ClipContentMatch[];
  copied: boolean;
  features: ClipPreviewFeatures;
  hiddenContentTypes: string[];
  isAddingNote: boolean;
  isManualTransformRunning: boolean;
  isTransforming: boolean;
  isWorkflowMenuOpen: boolean;
  onCopy: () => void;
  onCloseWorkflowMenu: () => void;
  onDelete: () => void;
  onManageTransforms?: () => void;
  onName: () => void;
  onPreviewTransform: (transform: SavedTransform) => void;
  onToggleConcealed: () => void;
  onToggleNote: () => void;
  onTogglePin: () => void;
  onToggleProtected: () => void;
  onToggleWorkflowMenu: () => void;
  protectedByBin: boolean;
  protectionToggleDisabled: boolean;
  provenance: ClipTransformationProvenance | null;
  transforms: SavedTransform[];
  trashEnabled: boolean;
  viewPolicy: ClipViewPolicy;
  visibleContentTypes: string[];
  workflowTriggerRef: RefObject<HTMLButtonElement | null>;
}) {
  return <div
    onMouseDown={startWindowDrag}
    onDoubleClick={handleWindowDragDoubleClick}
    className="col-preview-header h-[60px] px-4 flex items-center justify-between cursor-default titlebar-drag-handle shrink-0"
  >
    <div className="me-3 flex min-w-0 flex-1 flex-nowrap items-center gap-3 overflow-hidden whitespace-nowrap titlebar-drag-handle">
      {features.clipTypes && <span className="clip-type-badge theme-badge shrink-0 whitespace-nowrap text-xs font-semibold px-2.5 py-1 rounded-md border capitalize titlebar-drag-handle">
        {clip.content_type === 'file' && getClipFilePaths(clip).length > 1
          ? translate('component.clipPreview.files')
          : contentTypeLabel(structuralClipType(clip.content_type))}
      </span>}
      {features.types && visibleContentTypes.map((contentType) => <span
        key={contentType}
        title={contentMatchTitle(contentType, contentMatches)}
        className="clip-type-badge theme-badge shrink-0 whitespace-nowrap text-xs font-semibold px-2.5 py-1 rounded-md border titlebar-drag-handle"
      >
        {contentTypeLabel(contentType)}
      </span>)}
      {features.types && hiddenContentTypes.length > 0 && <span
        title={hiddenContentTypes.map(contentTypeLabel).join(', ')}
        className="clip-type-badge theme-badge shrink-0 whitespace-nowrap text-xs font-semibold px-2.5 py-1 rounded-md border titlebar-drag-handle"
      >
        +{hiddenContentTypes.length}
      </span>}
      {features.sources && <OverflowText
        text={localizedSourceName(clip.source)}
        className="theme-text-main min-w-0 max-w-[200px] truncate text-xs font-medium titlebar-drag-handle"
      />}
      {isTransforming && <LoaderCircle
        className="clip-transform-working h-4 w-4 shrink-0 animate-spin"
        aria-label={translate('component.clipPreview.applyingTransform')}
      />}
      {features.transformations && !isTransforming && provenance && <Workflow
        className="transform-accent manual-transforms h-4 w-4 shrink-0"
        aria-label={translate('component.clipPreview.transformedWithTransformname', { transformName: provenance.transformName })}
      />}
      {features.transformations && !isTransforming && provenance?.connectionId && <Sparkles
        className="transform-accent manual-transforms h-3.5 w-3.5 shrink-0"
        aria-label={translate('component.clipPreview.transformUsedConnectedIntelligence')}
      />}
    </div>

    <div className="clip-preview-actions relative flex shrink-0 items-center titlebar-no-drag">
      {features.transformations && viewPolicy.canRunManualTransforms && canTransformContent && <div className="clip-workflow-shell relative">
        <button
          ref={workflowTriggerRef}
          type="button"
          onClick={onToggleWorkflowMenu}
          className={`clip-preview-action clip-workflow-trigger theme-focusable transition-colors ${isWorkflowMenuOpen || activeTransformRef ? 'is-active' : ''}`}
          title={translate('component.clipPreview.workflow')}
          aria-label={translate('component.clipPreview.openClipWorkflow')}
          aria-haspopup="menu"
          aria-expanded={isWorkflowMenuOpen}
        >
          {isManualTransformRunning && activeTransformRef
            ? <LoaderCircle className="h-4 w-4 animate-spin" />
            : <Workflow className="h-4 w-4" />}
        </button>
        {isWorkflowMenuOpen && <ClipWorkflowMenu
          transforms={transforms}
          activeTransformRef={activeTransformRef}
          isRunning={isManualTransformRunning}
          anchorRef={workflowTriggerRef}
          onClose={onCloseWorkflowMenu}
          onPreview={onPreviewTransform}
          onManageTransforms={() => onManageTransforms?.()}
        />}
      </div>}
      <button
        type="button"
        onClick={onCopy}
        className={`clip-preview-action copy-clip-main-btn theme-focusable active:scale-95 transition-[background-color,color,transform] ${copied ? 'is-copied' : ''}`}
        title={copied ? UI_COPY.copied : UI_COPY.copy}
        aria-label={copied ? translate('component.clipPreview.clipCopied') : translate('component.clipPreview.copyClip')}
      >
        {copied ? <Check /> : <Copy />}
      </button>
      {features.concealment && viewPolicy.canOrganize && <button
        type="button"
        onClick={onToggleConcealed}
        className={`clip-preview-action preview-conceal-btn theme-focusable transition-colors ${concealmentEffective ? 'is-active' : ''}`}
        title={concealmentEffective ? translate('component.clipCard.revealSensitiveText') : translate('action.conceal')}
        aria-label={concealmentEffective ? translate('component.clipCard.revealSensitiveText') : translate('action.conceal')}
        aria-pressed={concealmentEffective}
      >
        {concealmentEffective ? <Eye /> : <EyeOff />}
      </button>}
      {features.naming && viewPolicy.canOrganize && <button
        type="button"
        onClick={onName}
        className={`clip-preview-action preview-name-btn theme-focusable transition-colors ${clip.name ? 'is-active' : ''}`}
        title={clip.name ? translate('action.editName') : translate('action.nameClip')}
        aria-label={clip.name ? translate('action.editName') : translate('action.nameClip')}
      >
        <FilePenLine />
      </button>}
      {viewPolicy.canOrganize && features.pinning && <button
        type="button"
        onClick={onTogglePin}
        className={`clip-preview-action preview-pin-btn theme-focusable transition-colors ${clip.is_pinned ? 'is-active' : ''}`}
        title={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
        aria-label={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
        aria-pressed={Boolean(clip.is_pinned)}
      >
        <Pin className={clip.is_pinned ? 'pin-icon' : ''} />
      </button>}
      {viewPolicy.canOrganize && features.protection && <button
        type="button"
        onClick={onToggleProtected}
        disabled={protectionToggleDisabled}
        className={`clip-preview-action preview-protect-btn theme-focusable transition-colors ${clip.is_protected ? 'is-active' : ''}`}
        title={clip.hotkey
          ? translate('component.clipCard.protectedByHotkey')
          : protectedByBin
            ? translate('component.clipPreview.protectedByBin')
            : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
        aria-label={clip.hotkey
          ? translate('component.clipCard.protectedByHotkey')
          : protectedByBin
            ? translate('component.clipPreview.protectedByBin')
            : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
        aria-pressed={Boolean(clip.is_protected)}
      >
        {clip.is_protected && !protectionToggleDisabled ? <ShieldOff /> : <Shield />}
      </button>}
      {features.notes && viewPolicy.canEditNotes && <button
        type="button"
        onClick={onToggleNote}
        className={`clip-preview-action preview-note-btn theme-focusable transition-colors ${isAddingNote ? 'is-active' : ''}`}
        title={isAddingNote ? translate('component.clipPreview.cancelNote') : translate('action.addNote')}
        aria-label={isAddingNote ? translate('component.clipPreview.cancelNote') : translate('action.addNote')}
        aria-pressed={isAddingNote}
      >
        <StickyNote />
      </button>}
      <button
        type="button"
        onClick={onDelete}
        disabled={Boolean(clip.is_protected) && viewPolicy.state !== 'trash'}
        className={`clip-preview-action preview-delete-btn theme-danger-text theme-focusable active:scale-95 transition-[background-color,color,opacity,transform] ${clip.is_protected && viewPolicy.state !== 'trash' ? 'cursor-not-allowed opacity-45' : ''}`}
        title={clip.is_protected && viewPolicy.state !== 'trash'
          ? translate('component.clipPreview.clipIsProtectedUnprotectFirstToDelete')
          : clipDeleteLabel({ trashEnabled, permanent: viewPolicy.state === 'trash' })}
        aria-label={viewPolicy.state === 'trash' || !trashEnabled
          ? translate('component.clipPreview.deleteClipPermanently')
          : translate('component.clipPreview.moveClipToTrash')}
      >
        {viewPolicy.state === 'trash' || !trashEnabled ? <X /> : <Trash2 />}
      </button>
    </div>
  </div>;
}
