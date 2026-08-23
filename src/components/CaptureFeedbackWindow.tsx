import { useEffect, useRef, useState, type WheelEvent } from 'react';
import { flushSync } from 'react-dom';
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
import { CaptureFeedbackCard } from './CaptureFeedbackCard';
import {
  CAPTURE_FEEDBACK_LAYOUT,
  MAX_CAPTURE_FEEDBACK_WINDOW_HEIGHT,
  type CaptureFeedbackClip,
  type CaptureFeedbackEvent,
  type CaptureFeedbackItem,
} from './captureFeedbackModel';

export type { CaptureFeedbackKind } from './captureFeedbackModel';

interface CaptureFeedbackWindowProps {
  settings: AppSettings;
  settingsHydrated: boolean;
}

const EXIT_DURATION_MS = 190;
const ENTER_DURATION_MS = 220;
const ENTER_PAINT_DELAY_MS = 64;
const DISPLAY_POLL_INTERVAL_MS = 180;
const HOVER_POLL_INTERVAL_MS = 60;
const PREVIEW_FADE_MS = 1_000;
const SWIPE_DISMISS_THRESHOLD = 54;
const STACK_COLLAPSE_MS = 160;

export function CaptureFeedbackWindow({ settings, settingsHydrated }: CaptureFeedbackWindowProps) {
  const [items, setItems] = useState<CaptureFeedbackItem[]>([]);
  const timers = useRef(new Map<number, number>());
  const eventSequence = useRef(0);
  const swipeState = useRef(new Map<number, { distance: number; time: number }>());
  const itemsRef = useRef<CaptureFeedbackItem[]>([]);
  const settingsRef = useRef(settings);
  const hydratedRef = useRef(settingsHydrated);
  const syncWindowRef = useRef<(nextItems: CaptureFeedbackItem[]) => Promise<void>>(async () => undefined);

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
    update: (current: CaptureFeedbackItem[]) => CaptureFeedbackItem[],
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

  const handleSwipe = (id: number, event: WheelEvent<HTMLDivElement>) => {
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

    const syncWindow = async (nextItems: CaptureFeedbackItem[]) => {
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
          await windowHandle.setSize(new LogicalSize(
            CAPTURE_FEEDBACK_LAYOUT.windowWidth,
            MAX_CAPTURE_FEEDBACK_WINDOW_HEIGHT,
          ));
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

      const item: CaptureFeedbackItem = {
        id: itemId,
        kind: event.kind,
        clip,
        image,
        entering: true,
      };
      const nextItems = commitItems((current) => [item, ...current]
        .sort((left, right) => right.id - left.id)
        .slice(0, CAPTURE_FEEDBACK_LAYOUT.maxStackItems));

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
        if (payload && ['success', 'ignored', 'failure'].includes(payload.kind)) void positionAndShow(payload);
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
  const updateItem = (id: number, update: (item: CaptureFeedbackItem) => CaptureFeedbackItem) => {
    commitItems((current) => current.map((item) => item.id === id ? update(item) : item));
  };
  const notifyLibraryChanged = () => void emit('clip-library-changed');
  const togglePinned = async (item: CaptureFeedbackItem) => {
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
  const toggleProtected = async (item: CaptureFeedbackItem) => {
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
  const removeClip = async (item: CaptureFeedbackItem) => {
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
      {items.map((item) => <CaptureFeedbackCard
        key={item.id}
        item={item}
        settings={settings}
        onSwipe={(event) => handleSwipe(item.id, event)}
        onTogglePinned={() => void togglePinned(item)}
        onToggleProtected={() => void toggleProtected(item)}
        onRemove={() => void removeClip(item)}
        onDismiss={() => dismiss(item.id)}
      />)}
    </div>
  );
}
