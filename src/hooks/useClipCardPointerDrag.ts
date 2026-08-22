import { useEffect, useRef, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from 'react';

export function useClipCardPointerDrag({
  clipId,
  canDragClips,
  setDraggedClipId,
  onPointerDragStart,
  onPointerDragMove,
  onPointerDragEnd,
  onPointerDragCancel,
}: {
  clipId: number;
  canDragClips: boolean;
  setDraggedClipId?: (id: number | null) => void;
  onPointerDragStart?: (id: number) => void;
  onPointerDragMove?: (x: number, y: number) => void;
  onPointerDragEnd?: (x: number, y: number, id: number) => void;
  onPointerDragCancel?: () => void;
}) {
  const pointerDragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    active: boolean;
  } | null>(null);
  const removePointerListenersRef = useRef<(() => void) | null>(null);
  const suppressClickRef = useRef(false);

  useEffect(() => () => removePointerListenersRef.current?.(), []);

  const consumeSuppressedClick = (event: ReactMouseEvent) => {
    if (!suppressClickRef.current) return false;
    event.preventDefault();
    event.stopPropagation();
    suppressClickRef.current = false;
    return true;
  };

  const onPointerDown = (event: ReactPointerEvent) => {
    if (!canDragClips
      || event.button !== 0
      || (event.target as HTMLElement).closest('button, input, select, textarea, a')) return;
    pointerDragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      active: false,
    };

    const removeListeners = () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerEnd);
      window.removeEventListener('pointercancel', handlePointerCancel);
      window.removeEventListener('keydown', handleKeyDown);
      removePointerListenersRef.current = null;
    };

    const suppressClickUntilPointerRelease = () => {
      suppressClickRef.current = true;
      const clearSuppression = () => {
        window.removeEventListener('pointerup', clearSuppression);
        window.removeEventListener('pointercancel', clearSuppression);
        setTimeout(() => {
          suppressClickRef.current = false;
        }, 0);
      };
      window.addEventListener('pointerup', clearSuppression);
      window.addEventListener('pointercancel', clearSuppression);
    };

    const handlePointerMove = (pointerEvent: PointerEvent) => {
      const drag = pointerDragRef.current;
      if (!drag || drag.pointerId !== pointerEvent.pointerId) return;
      if (!drag.active
        && Math.hypot(pointerEvent.clientX - drag.startX, pointerEvent.clientY - drag.startY) >= 6) {
        drag.active = true;
        setDraggedClipId?.(clipId);
        onPointerDragStart?.(clipId);
      }
      if (drag.active) {
        pointerEvent.preventDefault();
        onPointerDragMove?.(pointerEvent.clientX, pointerEvent.clientY);
      }
    };

    const handlePointerEnd = (pointerEvent: PointerEvent) => {
      const drag = pointerDragRef.current;
      if (!drag || drag.pointerId !== pointerEvent.pointerId) return;
      pointerDragRef.current = null;
      removeListeners();
      if (!drag.active) return;
      suppressClickRef.current = true;
      onPointerDragEnd?.(pointerEvent.clientX, pointerEvent.clientY, clipId);
      setDraggedClipId?.(null);
      setTimeout(() => {
        suppressClickRef.current = false;
      }, 0);
    };

    const handlePointerCancel = (pointerEvent: PointerEvent) => {
      const drag = pointerDragRef.current;
      if (!drag || drag.pointerId !== pointerEvent.pointerId) return;
      pointerDragRef.current = null;
      removeListeners();
      setDraggedClipId?.(null);
      onPointerDragCancel?.();
    };

    const handleKeyDown = (keyboardEvent: KeyboardEvent) => {
      const drag = pointerDragRef.current;
      if (keyboardEvent.key !== 'Escape' || !drag?.active) return;
      keyboardEvent.preventDefault();
      keyboardEvent.stopPropagation();
      pointerDragRef.current = null;
      removeListeners();
      suppressClickUntilPointerRelease();
      setDraggedClipId?.(null);
      onPointerDragCancel?.();
    };

    removePointerListenersRef.current?.();
    removePointerListenersRef.current = removeListeners;
    window.addEventListener('pointermove', handlePointerMove, { passive: false });
    window.addEventListener('pointerup', handlePointerEnd);
    window.addEventListener('pointercancel', handlePointerCancel);
    window.addEventListener('keydown', handleKeyDown);
  };

  return { consumeSuppressedClick, onPointerDown };
}
