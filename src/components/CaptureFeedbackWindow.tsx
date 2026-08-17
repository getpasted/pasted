import { useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';
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
import { emit, listen } from '@tauri-apps/api/event';
import {
  availableMonitors,
  cursorPosition,
  currentMonitor,
  getCurrentWindow,
  LogicalSize,
  monitorFromPoint,
  PhysicalPosition,
  primaryMonitor,
} from '@tauri-apps/api/window';
import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { FloatingActionStrip } from './FloatingActionStrip';
import { SafeRasterImage } from './SafeRasterImage';
import { translate } from '../localization/runtime';

export type CaptureFeedbackKind = 'success' | 'ignored' | 'failure';

interface CaptureFeedbackWindowProps {
  settings: AppSettings;
  settingsHydrated: boolean;
}

interface CaptureFeedbackEvent {
  kind: CaptureFeedbackKind;
  clip_id?: number;
}

interface CaptureFeedbackClip {
  id: number;
  contentType: string;
  previewText: string | null;
  source: string;
  isPinned: boolean;
  isProtected: boolean;
  isTrashed: boolean;
}

interface FeedbackItem {
  id: number;
  kind: CaptureFeedbackKind;
  clip: CaptureFeedbackClip | null;
  image: string | null;
  entering?: boolean;
  exiting?: boolean;
  fading?: boolean;
  collapsing?: boolean;
  exitDirection?: -1 | 1;
}

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

const WINDOW_WIDTH = 340;
const PREVIEW_HEIGHT = 118;
const NOTICE_HEIGHT = 72;
const STACK_GAP = 6;
const WINDOW_PADDING = 6;
const MAX_STACK_ITEMS = 4;
const EXIT_DURATION_MS = 190;
const ENTER_DURATION_MS = 220;
const ENTER_PAINT_DELAY_MS = 64;
const DISPLAY_POLL_INTERVAL_MS = 180;
const HOVER_POLL_INTERVAL_MS = 60;
const PREVIEW_FADE_MS = 1_000;
const SWIPE_DISMISS_THRESHOLD = 54;
const STACK_COLLAPSE_MS = 160;
const MAX_WINDOW_HEIGHT = PREVIEW_HEIGHT * MAX_STACK_ITEMS
  + STACK_GAP * (MAX_STACK_ITEMS - 1)
  + WINDOW_PADDING * 2;

function contentIcon(contentType: string) {
  if (contentType === 'image') return ImageIcon;
  if (contentType === 'file') return File;
  return Type;
}

export function CaptureFeedbackWindow({ settings, settingsHydrated }: CaptureFeedbackWindowProps) {
  const [items, setItems] = useState<FeedbackItem[]>([]);
  const timers = useRef(new Map<number, number>());
  const eventSequence = useRef(0);
  const swipeState = useRef(new Map<number, { distance: number; time: number }>());
  const itemsRef = useRef<FeedbackItem[]>([]);
  const settingsRef = useRef(settings);
  const hydratedRef = useRef(settingsHydrated);
  const syncWindowRef = useRef<(nextItems: FeedbackItem[]) => Promise<void>>(async () => undefined);

  useEffect(() => {
    settingsRef.current = settings;
    hydratedRef.current = settingsHydrated;
    if (!settings.enableNotifications) {
      timers.current.forEach((timer) => window.clearTimeout(timer));
      timers.current.clear();
      itemsRef.current = [];
      setItems([]);
      void syncWindowRef.current([]);
      return;
    }
    if (itemsRef.current.length > 0) void syncWindowRef.current(itemsRef.current);
  }, [settings, settingsHydrated]);

  const commitItems = (
    update: (current: FeedbackItem[]) => FeedbackItem[],
    syncWindow = false,
  ) => {
    const next = update(itemsRef.current);
    itemsRef.current = next;
    if (syncWindow) {
      // Keep the rendered stack and its native WebView dimensions atomic.
      // Resizing before React commits briefly squeezes the old stack.
      flushSync(() => setItems(next));
    } else {
      setItems(next);
    }
    if (syncWindow) void syncWindowRef.current(next);
    return next;
  };

  const finishDismiss = (id: number) => {
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    timers.current.delete(id);
    commitItems((current) => current.filter((item) => item.id !== id), true);
  };

  const beginCollapse = (id: number) => {
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    commitItems((current) => current.map((candidate) => candidate.id === id
      ? { ...candidate, collapsing: true }
      : candidate));
    timers.current.set(id, window.setTimeout(() => finishDismiss(id), STACK_COLLAPSE_MS));
  };

  const dismiss = (id: number, exitDirection?: -1 | 1) => {
    const item = itemsRef.current.find((candidate) => candidate.id === id);
    if (!item || item.exiting) return;
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    const direction = exitDirection
      ?? (settingsRef.current.captureFeedbackPosition.endsWith('right') ? 1 : -1);
    commitItems((current) => current.map((candidate) => candidate.id === id
      ? { ...candidate, fading: false, exiting: true, exitDirection: direction }
      : candidate));
    timers.current.set(id, window.setTimeout(() => beginCollapse(id), EXIT_DURATION_MS));
  };

  const collapseAfterFade = (id: number) => {
    const item = itemsRef.current.find((candidate) => candidate.id === id);
    if (!item || item.exiting) return;
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    commitItems((current) => current.map((candidate) => candidate.id === id
      ? { ...candidate, fading: true, exiting: true, collapsing: true }
      : candidate));
    timers.current.set(id, window.setTimeout(() => finishDismiss(id), STACK_COLLAPSE_MS));
  };

  const pauseAutoDismiss = (id: number) => {
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    timers.current.delete(id);
    const item = itemsRef.current.find((candidate) => candidate.id === id);
    if (item?.fading) {
      commitItems((current) => current.map((candidate) => candidate.id === id
        ? { ...candidate, fading: false }
        : candidate));
    }
  };

  const scheduleAutoDismiss = (id: number) => {
    const item = itemsRef.current.find((candidate) => candidate.id === id);
    if (!item?.clip || item.exiting) return;
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    timers.current.delete(id);
    commitItems((current) => current.map((candidate) => candidate.id === id && candidate.fading
      ? { ...candidate, fading: false }
      : candidate));
    if (item.clip.isPinned) return;
    const delaySeconds = settingsRef.current.captureFeedbackDismissSeconds;
    if (delaySeconds <= 0) return;
    timers.current.set(id, window.setTimeout(() => {
      commitItems((current) => current.map((candidate) => candidate.id === id
        ? { ...candidate, fading: true }
        : candidate));
      timers.current.set(id, window.setTimeout(() => collapseAfterFade(id), PREVIEW_FADE_MS));
    }, delaySeconds * 1_000));
  };

  const handleSwipe = (id: number, event: React.WheelEvent<HTMLDivElement>) => {
    if (Math.abs(event.deltaX) <= Math.abs(event.deltaY) * 1.15) return;
    const now = performance.now();
    const previous = swipeState.current.get(id);
    const distance = previous && now - previous.time < 400
      ? previous.distance + Math.abs(event.deltaX)
      : Math.abs(event.deltaX);
    swipeState.current.set(id, { distance, time: now });
    if (distance < SWIPE_DISMISS_THRESHOLD) return;
    swipeState.current.delete(id);
    dismiss(id, event.deltaX >= 0 ? 1 : -1);
  };

  useEffect(() => {
    const windowHandle = getCurrentWindow();
    let disposed = false;
    let placementBusy = false;
    let placementQueuedForce = false;
    let lastDisplayKey = '';
    let lastCursorIcon: 'arrow' | 'hand' = 'arrow';
    let lastHoveredItemId: number | null = null;
    let lastIgnoreCursorEvents: boolean | null = null;
    let nativeWindowVisible = false;
    const unlisteners: Array<() => void> = [];

    const setWindowCursorPassthrough = (ignore: boolean) => {
      if (lastIgnoreCursorEvents === ignore) return;
      lastIgnoreCursorEvents = ignore;
      void windowHandle.setIgnoreCursorEvents(ignore).catch((error) => {
        lastIgnoreCursorEvents = null;
        console.warn('Could not update capture feedback interaction mode:', error);
      });
    };

    const setSyntheticCursor = (icon: 'arrow' | 'hand') => {
      document.documentElement.style.cursor = icon === 'hand' ? 'pointer' : '';
      const changed = lastCursorIcon !== icon;
      if (changed) {
        lastCursorIcon = icon;
        void windowHandle.setCursorIcon(icon).catch(() => undefined);
      }
      // WebKit cursor rects are unreliable in a non-focusable macOS overlay,
      // but repeatedly asserting AppKit's cursor makes the two systems flap.
      // Cross the native cursor boundary exactly once in each direction.
      if (changed) {
        void invoke('set_overlay_cursor', { pointing: icon === 'hand' });
      }
    };

    const clearSyntheticHover = () => {
      document.querySelectorAll('.is-global-pointer-hover').forEach((element) => {
        element.classList.remove('is-global-pointer-hover');
      });
      setSyntheticCursor('arrow');
    };

    const syncSyntheticHover = async () => {
      if (!itemsRef.current.some((item) => item.clip)) {
        clearSyntheticHover();
        setWindowCursorPassthrough(true);
        return;
      }
      try {
        const [pointer, origin] = await Promise.all([cursorPosition(), windowHandle.outerPosition()]);
        const scale = window.devicePixelRatio || 1;
        const localX = (pointer.x - origin.x) / scale;
        const localY = (pointer.y - origin.y) / scale;
        const hit = document.elementFromPoint(localX, localY);
        const hoveredCard = hit?.closest('.capture-feedback-card.is-preview') ?? null;
        const hoveredAction = hit?.closest('.floating-action-button:not(:disabled)') ?? null;
        const hoveredItemId = hoveredCard instanceof HTMLElement
          ? Number(hoveredCard.dataset.feedbackId)
          : null;

        if (hoveredItemId !== lastHoveredItemId) {
          if (lastHoveredItemId !== null) scheduleAutoDismiss(lastHoveredItemId);
          if (hoveredItemId !== null && Number.isFinite(hoveredItemId)) pauseAutoDismiss(hoveredItemId);
          lastHoveredItemId = hoveredItemId;
        }

        document.querySelectorAll('.capture-feedback-card.is-preview').forEach((element) => {
          element.classList.toggle('is-global-pointer-hover', element === hoveredCard);
        });
        document.querySelectorAll('.capture-feedback-card .floating-action-button').forEach((element) => {
          element.classList.toggle('is-global-pointer-hover', element === hoveredAction);
        });
        setWindowCursorPassthrough(!hoveredCard);
        setSyntheticCursor(hoveredAction ? 'hand' : 'arrow');
      } catch {
        clearSyntheticHover();
        setWindowCursorPassthrough(true);
      }
    };

    const placeOnPointerDisplay = async (force = false) => {
      if (itemsRef.current.length === 0) return;
      if (placementBusy) {
        placementQueuedForce ||= force;
        return;
      }
      placementBusy = true;
      try {
        let monitor = null;
        try {
          const pointer = await cursorPosition();
          const monitors = await availableMonitors();
          monitor = monitors.find((candidate) => {
            const area = candidate.workArea;
            return pointer.x >= area.position.x
              && pointer.x < area.position.x + area.size.width
              && pointer.y >= area.position.y
              && pointer.y < area.position.y + area.size.height;
          }) ?? await monitorFromPoint(pointer.x, pointer.y);
        } catch (error) {
          console.warn('Could not resolve the pointer display for capture feedback:', error);
        }
        monitor ??= await currentMonitor() ?? await primaryMonitor();
        if (!monitor) return;

        const position = settingsRef.current.captureFeedbackPosition;
        const displayKey = [
          monitor.position.x,
          monitor.position.y,
          monitor.size.width,
          monitor.size.height,
          monitor.scaleFactor,
          position,
        ].join(':');
        if (!force && displayKey === lastDisplayKey) return;

        const outerSize = await windowHandle.outerSize();
        const inset = Math.round(18 * monitor.scaleFactor);
        const x = position.endsWith('left')
          ? monitor.position.x + inset
          : monitor.position.x + monitor.size.width - outerSize.width - inset;
        const y = position.startsWith('top')
          ? monitor.position.y + inset
          : monitor.position.y + monitor.size.height - outerSize.height - inset;
        await windowHandle.setPosition(new PhysicalPosition(x, y));
        lastDisplayKey = displayKey;
      } finally {
        placementBusy = false;
        if (placementQueuedForce) {
          placementQueuedForce = false;
          void placeOnPointerDisplay(true);
        }
      }
    };

    const syncWindow = async (nextItems: FeedbackItem[]) => {
      if (nextItems.length === 0) {
        lastDisplayKey = '';
        clearSyntheticHover();
        setWindowCursorPassthrough(true);
        await windowHandle.hide();
        nativeWindowVisible = false;
        return;
      }

      try {
        await windowHandle.setFocusable(false);
        if (!nativeWindowVisible) {
          await windowHandle.setSize(new LogicalSize(WINDOW_WIDTH, MAX_WINDOW_HEIGHT));
        }
        await placeOnPointerDisplay(true);
      } catch (error) {
        console.warn('Could not position capture feedback:', error);
      }

      setWindowCursorPassthrough(true);
      if (!nativeWindowVisible) {
        await windowHandle.show();
        nativeWindowVisible = true;
      }
    };
    syncWindowRef.current = syncWindow;
    const displayPoll = window.setInterval(() => {
      if (itemsRef.current.length > 0) void placeOnPointerDisplay();
    }, DISPLAY_POLL_INTERVAL_MS);
    const hoverPoll = window.setInterval(() => {
      if (itemsRef.current.length > 0) void syncSyntheticHover();
    }, HOVER_POLL_INTERVAL_MS);

    const positionAndShow = async (event: CaptureFeedbackEvent) => {
      const itemId = ++eventSequence.current;
      const currentSettings = settingsRef.current;
      if (!hydratedRef.current || !currentSettings.enableNotifications || !currentSettings.captureFeedback) return;
      if (event.kind === 'ignored' && !currentSettings.captureFeedbackIgnored) return;

      let clip: CaptureFeedbackClip | null = null;
      let image: string | null = null;
      const showClip = event.kind === 'success'
        && Boolean(event.clip_id)
        && currentSettings.captureFeedbackPreview;

      if (showClip && event.clip_id) {
        try {
          clip = await invoke<CaptureFeedbackClip>('get_capture_feedback_clip', { id: event.clip_id });
          if (clip.contentType === 'image') {
            image = await invoke<string | null>('get_clip_image', { id: event.clip_id });
          }
        } catch (error) {
          console.error('Failed to prepare capture preview:', error);
        }
      }
      if (disposed) return;

      const item: FeedbackItem = {
        id: itemId,
        kind: event.kind,
        clip,
        image,
        entering: true,
      };
      const nextItems = commitItems((current) => [item, ...current]
        .sort((left, right) => right.id - left.id)
        .slice(0, MAX_STACK_ITEMS));

      await syncWindowRef.current(nextItems);
      // Clear the entrance phase only after its CSS animation has completed.
      // This timer never gates window visibility; at worst animation is skipped.
      window.setTimeout(() => {
        if (disposed) return;
        commitItems((current) => current.map((candidate) => candidate.id === item.id
          ? { ...candidate, entering: false }
          : candidate));
      }, ENTER_PAINT_DELAY_MS + ENTER_DURATION_MS);

      if (!clip) {
        const timer = window.setTimeout(() => dismiss(item.id), 1_800);
        timers.current.set(item.id, timer);
      } else {
        scheduleAutoDismiss(item.id);
      }
    };

    const register = async () => {
      unlisteners.push(await listen<CaptureFeedbackEvent>('clipboard-capture-feedback', ({ payload }) => {
        if (payload && payload.kind in FEEDBACK) void positionAndShow(payload);
      }));
    };

    void register().then(() => {
      if (!disposed) return;
      unlisteners.splice(0).forEach((unlisten) => unlisten());
    });

    return () => {
      disposed = true;
      timers.current.forEach((timer) => window.clearTimeout(timer));
      timers.current.clear();
      window.clearInterval(displayPoll);
      window.clearInterval(hoverPoll);
      clearSyntheticHover();
      unlisteners.splice(0).forEach((unlisten) => unlisten());
    };
  }, []);
  const updateItem = (id: number, update: (item: FeedbackItem) => FeedbackItem) => {
    commitItems((current) => current.map((item) => item.id === id ? update(item) : item));
  };
  const notifyLibraryChanged = () => void emit('clip-library-changed');
  const togglePinned = async (item: FeedbackItem) => {
    if (!item.clip) return;
    try {
      const isPinned = await invoke<boolean>('toggle_pin_clip', { id: item.clip.id });
      updateItem(item.id, (current) => current.clip
        ? { ...current, clip: { ...current.clip, isPinned } }
        : current);
      if (isPinned) pauseAutoDismiss(item.id);
      else scheduleAutoDismiss(item.id);
      notifyLibraryChanged();
    } catch (error) {
      console.error('Could not update pin from capture feedback:', error);
    }
  };
  const toggleProtected = async (item: FeedbackItem) => {
    if (!item.clip) return;
    try {
      const isProtected = await invoke<boolean>('toggle_clip_protected', { clipId: item.clip.id });
      updateItem(item.id, (current) => current.clip
        ? { ...current, clip: { ...current.clip, isProtected } }
        : current);
      notifyLibraryChanged();
    } catch (error) {
      console.error('Could not update protection from capture feedback:', error);
    }
  };
  const removeClip = async (item: FeedbackItem) => {
    if (!item.clip || item.clip.isProtected) return;
    try {
      if (settings.enableTrash && !item.clip.isTrashed) await invoke('delete_clip', { id: item.clip.id });
      else await invoke('purge_clip_permanently', { id: item.clip.id });
      notifyLibraryChanged();
      dismiss(item.id);
    } catch (error) {
      console.error('Could not remove clip from capture feedback:', error);
    }
  };

  const bottomStack = settings.captureFeedbackPosition.startsWith('bottom');

  return (
    <div className={`capture-feedback-root flex h-screen w-screen gap-1.5 p-1.5 ${bottomStack ? 'is-bottom-stack flex-col-reverse' : 'flex-col'}`}>
      {items.map((item) => {
        const feedback = FEEDBACK[item.kind];
        const Icon = feedback.Icon;
        const ContentIcon = item.clip ? contentIcon(item.clip.contentType) : Type;
        return (
          <div
            key={item.id}
            className={`capture-feedback-slot w-full shrink-0 ${item.collapsing ? 'is-collapsing' : ''}`}
            style={{ '--capture-feedback-card-height': `${item.clip ? PREVIEW_HEIGHT : NOTICE_HEIGHT}px` } as React.CSSProperties}
          >
          <div
            className={`capture-feedback-card flex h-full w-full flex-col rounded-xl border shadow-xl ${item.entering ? 'is-entering' : ''} ${item.exiting ? 'is-exiting' : ''} ${item.fading ? 'is-auto-fading' : ''} ${item.clip ? 'clip-card-idle is-preview relative' : `theme-status-${feedback.tone}`}`}
            data-feedback-id={item.id}
            style={{ '--capture-feedback-exit-x': `${(item.exitDirection ?? 1) * 24}px` } as React.CSSProperties}
            onMouseDown={(event) => event.preventDefault()}
            onWheel={item.clip ? (event) => handleSwipe(item.id, event) : undefined}
          >
            {item.clip ? (
              <>
                <div className="flex min-h-0 flex-1 gap-2.5 p-3 pb-10">
                  <span className="clip-type-icon theme-badge flex h-7 w-7 shrink-0 items-center justify-center rounded border">
                    <ContentIcon className="h-3.5 w-3.5" aria-hidden="true" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-xs font-semibold theme-text-main">{settings.enableSources ? item.clip.source || translate('component.captureFeedbackWindow.capturedClip') : translate('component.captureFeedbackWindow.capturedClip')}</div>
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
                    <button type="button" className={`floating-action-button ${item.clip.isPinned ? 'is-success pin-icon' : ''}`} onClick={() => void togglePinned(item)} title={item.clip.isPinned ? translate('action.unpin') : translate('action.pin')} aria-label={item.clip.isPinned ? translate('component.captureFeedbackWindow.unpinClip') : translate('component.captureFeedbackWindow.pinClip')}>
                      <Pin aria-hidden="true" />
                    </button>
                  )}
                  {settings.enableProtection && (
                    <button type="button" className={`floating-action-button ${item.clip.isProtected ? 'is-accent' : ''}`} onClick={() => void toggleProtected(item)} title={item.clip.isProtected ? translate('action.unprotect') : translate('action.protect')} aria-label={item.clip.isProtected ? translate('component.captureFeedbackWindow.unprotectClip') : translate('component.captureFeedbackWindow.protectClip')}>
                      {item.clip.isProtected ? <ShieldOff aria-hidden="true" /> : <Shield aria-hidden="true" />}
                    </button>
                  )}
                  <button type="button" className="floating-action-button is-danger" disabled={item.clip.isProtected || item.clip.isTrashed} onClick={() => void removeClip(item)} title={settings.enableTrash ? translate('action.moveToTrash') : translate('common.delete')} aria-label={settings.enableTrash ? translate('component.captureFeedbackWindow.moveClipToTrash') : translate('component.captureFeedbackWindow.deleteClip')}>
                    <Trash2 aria-hidden="true" />
                  </button>
                  <button type="button" className="floating-action-button" onClick={() => dismiss(item.id)} title={translate('common.dismiss')} aria-label={translate('component.captureFeedbackWindow.dismissPreview')}>
                    <X aria-hidden="true" />
                  </button>
                </FloatingActionStrip>
              </>
            ) : (
              <div className="flex items-center gap-3 px-3.5 py-3">
                <span className="capture-feedback-icon flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border">
                  <Icon className="h-4 w-4" aria-hidden="true" />
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
      })}
    </div>
  );
}
