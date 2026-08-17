import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { scheduleBackupClientStatePersistence } from '../utils/backupClientState';
import { useLocalization } from '../localization/LocalizationProvider';
import { inlineResizeDelta } from '../utils/direction';

const SIDEBAR_KEY = 'pasted_sidebar_width';
const LIST_KEY = 'pasted_list_width';
const SIDEBAR_DEFAULT = 240;
const LIST_DEFAULT = 340;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function readWidth(key: string, fallback: number, min: number, max: number) {
  const parsed = Number.parseInt(localStorage.getItem(key) ?? '', 10);
  return Number.isFinite(parsed) ? clamp(parsed, min, max) : fallback;
}

export function useColumnResize() {
  const { direction } = useLocalization();
  const [sidebarWidth, setSidebarWidth] = useState(() => readWidth(SIDEBAR_KEY, SIDEBAR_DEFAULT, 180, 360));
  const [clipsListWidth, setClipsListWidth] = useState(() => readWidth(LIST_KEY, LIST_DEFAULT, 280, 520));
  const [activeColumn, setActiveColumn] = useState<'sidebar' | 'list' | null>(null);
  const sidebarWidthRef = useRef(sidebarWidth);
  const clipsListWidthRef = useRef(clipsListWidth);
  const activeCleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    sidebarWidthRef.current = sidebarWidth;
  }, [sidebarWidth]);

  useEffect(() => {
    clipsListWidthRef.current = clipsListWidth;
  }, [clipsListWidth]);

  const startResize = useCallback((
    event: ReactPointerEvent<HTMLDivElement>,
    column: 'sidebar' | 'list',
  ) => {
    if (event.button !== 0) return;
    event.preventDefault();
    activeCleanupRef.current?.();

    const handle = event.currentTarget;
    const pointerId = event.pointerId;
    const startX = event.clientX;
    const startWidth = column === 'sidebar' ? sidebarWidthRef.current : clipsListWidthRef.current;
    const min = column === 'sidebar' ? 180 : 280;
    const max = column === 'sidebar' ? 360 : 520;
    const storageKey = column === 'sidebar' ? SIDEBAR_KEY : LIST_KEY;
    let finalWidth = startWidth;
    let finished = false;

    handle.setPointerCapture(pointerId);
    setActiveColumn(column);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      finalWidth = clamp(startWidth + inlineResizeDelta(startX, moveEvent.clientX, direction), min, max);
      if (column === 'sidebar') {
        sidebarWidthRef.current = finalWidth;
        setSidebarWidth(finalWidth);
      } else {
        clipsListWidthRef.current = finalWidth;
        setClipsListWidth(finalWidth);
      }
    };

    const finish = () => {
      if (finished) return;
      finished = true;
      localStorage.setItem(storageKey, String(finalWidth));
      scheduleBackupClientStatePersistence();
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', finish);
      window.removeEventListener('pointercancel', finish);
      window.removeEventListener('blur', finish);
      if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
      setActiveColumn(null);
      activeCleanupRef.current = null;
    };

    activeCleanupRef.current = finish;
    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', finish);
    window.addEventListener('pointercancel', finish);
    window.addEventListener('blur', finish);
  }, [direction]);

  useEffect(() => () => activeCleanupRef.current?.(), []);

  const resetColumnWidths = useCallback(() => {
    activeCleanupRef.current?.();
    sidebarWidthRef.current = SIDEBAR_DEFAULT;
    clipsListWidthRef.current = LIST_DEFAULT;
    setSidebarWidth(SIDEBAR_DEFAULT);
    setClipsListWidth(LIST_DEFAULT);
    localStorage.removeItem(SIDEBAR_KEY);
    localStorage.removeItem(LIST_KEY);
    scheduleBackupClientStatePersistence();
  }, []);

  return {
    sidebarWidth,
    clipsListWidth,
    isResizingSidebar: activeColumn === 'sidebar',
    isResizingList: activeColumn === 'list',
    handleSidebarPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => startResize(event, 'sidebar'),
    handleListPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => startResize(event, 'list'),
    resetColumnWidths,
  };
}
