import type { CSSProperties, WheelEvent } from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  EyeOff,
  File,
  Image as ImageIcon,
  Pin,
  Shield,
  ShieldOff,
  Trash2,
  Type,
  X,
} from 'lucide-react';
import type { AppSettings } from '../types';
import { translate } from '../localization/runtime';
import { FloatingActionStrip } from './FloatingActionStrip';
import { SafeRasterImage } from './SafeRasterImage';
import { CAPTURE_FEEDBACK_LAYOUT, type CaptureFeedbackItem } from './captureFeedbackModel';

const FEEDBACK = {
  success: {
    get title() { return translate('component.captureFeedbackWindow.savedToHistory'); },
    get detail() { return translate('component.captureFeedbackWindow.readyOnDemand'); },
    Icon: CheckCircle2,
    tone: 'success',
  },
  ignored: {
    get title() { return translate('component.captureFeedbackWindow.captureSkipped'); },
    get detail() { return translate('component.captureFeedbackWindow.thisClipboardItemWasLeftAlone'); },
    Icon: EyeOff,
    tone: 'info',
  },
  failure: {
    get title() { return translate('component.captureFeedbackWindow.captureFailed'); },
    get detail() { return translate('component.captureFeedbackWindow.thisClipboardItemCouldNotBeSaved'); },
    Icon: AlertTriangle,
    tone: 'danger',
  },
} as const;

function contentIcon(contentType: string) {
  if (contentType === 'image') return ImageIcon;
  if (contentType === 'file') return File;
  return Type;
}

interface CaptureFeedbackCardProps {
  item: CaptureFeedbackItem;
  settings: AppSettings;
  onSwipe: (event: WheelEvent<HTMLDivElement>) => void;
  onTogglePinned: () => void;
  onToggleProtected: () => void;
  onRemove: () => void;
  onDismiss: () => void;
}

export function CaptureFeedbackCard({
  item,
  settings,
  onSwipe,
  onTogglePinned,
  onToggleProtected,
  onRemove,
  onDismiss,
}: CaptureFeedbackCardProps) {
  const feedback = FEEDBACK[item.kind];
  const FeedbackIcon = feedback.Icon;
  const ContentIcon = item.clip ? contentIcon(item.clip.contentType) : Type;
  const cardHeight = item.clip
    ? CAPTURE_FEEDBACK_LAYOUT.previewHeight
    : CAPTURE_FEEDBACK_LAYOUT.noticeHeight;

  return (
    <div className={`capture-feedback-slot w-full shrink-0 ${item.collapsing ? 'is-collapsing' : ''}`} style={{ '--capture-feedback-card-height': `${cardHeight}px` } as CSSProperties}>
      <div
        className={`capture-feedback-card elevation-floating flex h-full w-full flex-col rounded-xl border ${item.entering ? 'is-entering' : ''} ${item.exiting ? 'is-exiting' : ''} ${item.fading ? 'is-auto-fading' : ''} ${item.clip ? 'clip-card-idle is-preview relative' : `theme-status-${feedback.tone}`}`}
        data-feedback-id={item.id}
        style={{ '--capture-feedback-exit-x': `${(item.exitDirection ?? 1) * 24}px` } as CSSProperties}
        onMouseDown={(event) => event.preventDefault()}
        onWheel={item.clip ? onSwipe : undefined}
      >
        {item.clip ? (
          <>
            <div className="flex min-h-0 flex-1 gap-2.5 p-3 pb-10">
              <span className="clip-type-icon theme-badge flex h-7 w-7 shrink-0 items-center justify-center rounded border">
                <ContentIcon className="h-3.5 w-3.5" aria-hidden="true" />
              </span>
              <div className="min-w-0 flex-1">
                <div className="truncate text-xs font-semibold theme-text-main">
                  {settings.enableSources ? item.clip.source || translate('component.captureFeedbackWindow.capturedClip') : translate('component.captureFeedbackWindow.capturedClip')}
                </div>
                <div className="capture-feedback-preview mt-1.5 min-h-0 overflow-hidden rounded-md">
                  {item.image ? (
                    <SafeRasterImage source={item.image} alt={translate('component.captureFeedbackWindow.capturedClipPreview')} className="h-11 w-full object-cover" />
                  ) : (
                    <p className="line-clamp-2 w-full break-words px-2 py-1.5 font-mono text-[10px] leading-[1.35]">
                      {item.clip.previewText || translate('component.captureFeedbackWindow.previewUnavailableForThisClipType')}
                    </p>
                  )}
                </div>
              </div>
            </div>
            <FloatingActionStrip label={translate('component.captureFeedbackWindow.capturedClipActions')}>
              {settings.enablePinning && (
                <button type="button" className={`floating-action-button ${item.clip.isPinned ? 'is-success pin-icon' : ''}`} onClick={onTogglePinned} title={item.clip.isPinned ? translate('action.unpin') : translate('action.pin')} aria-label={item.clip.isPinned ? translate('component.captureFeedbackWindow.unpinClip') : translate('component.captureFeedbackWindow.pinClip')}>
                  <Pin aria-hidden="true" />
                </button>
              )}
              {settings.enableProtection && (
                <button type="button" className={`floating-action-button ${item.clip.isProtected ? 'is-accent' : ''}`} onClick={onToggleProtected} title={item.clip.isProtected ? translate('action.unprotect') : translate('action.protect')} aria-label={item.clip.isProtected ? translate('component.captureFeedbackWindow.unprotectClip') : translate('component.captureFeedbackWindow.protectClip')}>
                  {item.clip.isProtected ? <ShieldOff aria-hidden="true" /> : <Shield aria-hidden="true" />}
                </button>
              )}
              <button type="button" className="floating-action-button is-danger" disabled={item.clip.isProtected || item.clip.isTrashed} onClick={onRemove} title={settings.enableTrash ? translate('action.moveToTrash') : translate('common.delete')} aria-label={settings.enableTrash ? translate('component.captureFeedbackWindow.moveClipToTrash') : translate('component.captureFeedbackWindow.deleteClip')}>
                <Trash2 aria-hidden="true" />
              </button>
              <button type="button" className="floating-action-button" onClick={onDismiss} title={translate('common.dismiss')} aria-label={translate('component.captureFeedbackWindow.dismissPreview')}>
                <X aria-hidden="true" />
              </button>
            </FloatingActionStrip>
          </>
        ) : (
          <div className="flex items-center gap-3 px-3.5 py-3">
            <span className="capture-feedback-icon flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border">
              <FeedbackIcon className="h-4 w-4" aria-hidden="true" />
            </span>
            <div className="min-w-0">
              <div className="text-xs font-bold leading-tight">{feedback.title}</div>
              <div className="mt-0.5 truncate text-[10px] font-medium opacity-75">{feedback.detail}</div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
