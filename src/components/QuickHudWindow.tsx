import React, { useState, useEffect, useRef } from 'react';
import { AlertCircle, LoaderCircle, Search, Sparkles, X } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { listen } from '@tauri-apps/api/event';
import { ClipItem, getClipFileSummary } from '../types';
import { OverflowText } from './OverflowText';
import { SafeRasterImage } from './SafeRasterImage';
import { translate } from '../localization/runtime';

export const QuickHudWindow: React.FC = () => {
  const [hudAnchor, setHudAnchor] = useState({ flipped: false, x: 180 });
  const [clips, setClips] = useState<ClipItem[]>(() => {
    try {
      const cached = localStorage.getItem('pasted_cache_hud_clips');
      return cached ? JSON.parse(cached) : [];
    } catch {
      return [];
    }
  });
  const [search, setSearch] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [pasteError, setPasteError] = useState('');
  const [isPasting, setIsPasting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const isPastingRef = useRef(false);
  const clipsRef = useRef(clips);
  const selectedIndexRef = useRef(selectedIndex);
  clipsRef.current = clips;
  selectedIndexRef.current = selectedIndex;

  const fetchClips = async () => {
    try {
      const result = await invoke<ClipItem[]>('get_clips', {
        searchQuery: search || null,
        binId: null,
        onlyPinned: false,
        limit: 9,
        offset: 0,
      });
      setClips(result);
      setSelectedIndex(0);
      if (!search) {
        try {
          localStorage.setItem('pasted_cache_hud_clips', JSON.stringify(result));
        } catch {
          // Ignore
        }
      }
    } catch (e) {
      console.error('Failed to fetch clips for HUD:', e);
    }
  };
  const fetchClipsRef = useRef(fetchClips);
  fetchClipsRef.current = fetchClips;

  const activateClip = async (clip: ClipItem) => {
    if (isPastingRef.current) return;
    isPastingRef.current = true;
    setIsPasting(true);
    setPasteError('');
    try {
      await invoke('paste_clip_by_id', { clipId: clip.id });
    } catch (error) {
      setPasteError(error instanceof Error ? error.message : String(error));
    } finally {
      isPastingRef.current = false;
      setIsPasting(false);
    }
  };

  const handleHudKeyDown = async (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      await invoke('toggle_hud_window');
      return;
    }
    if (/^[1-9]$/.test(e.key) && !e.metaKey && !e.ctrlKey && !e.altKey) {
      const idx = parseInt(e.key, 10) - 1;
      const clip = clipsRef.current[idx];
      if (clip) {
        e.preventDefault();
        setSelectedIndex(idx);
        await activateClip(clip);
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev + 1) % Math.max(1, clipsRef.current.length));
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev - 1 + clipsRef.current.length) % Math.max(1, clipsRef.current.length));
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      const selectedClip = clipsRef.current[selectedIndexRef.current];
      if (selectedClip) await activateClip(selectedClip);
    }
  };

  useEffect(() => {
    fetchClips();
  }, [search]);

  useEffect(() => {
    const selectedRow = listRef.current?.querySelector<HTMLElement>(
      `[data-hud-index="${selectedIndex}"]`,
    );
    selectedRow?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  useEffect(() => {
    document.documentElement.classList.add('hud-mode');
    document.body.classList.add('hud-mode');
    const rootEl = document.getElementById('root');
    if (rootEl) rootEl.classList.add('hud-mode');

    return () => {
      document.documentElement.classList.remove('hud-mode');
      document.body.classList.remove('hud-mode');
      if (rootEl) rootEl.classList.remove('hud-mode');
    };
  }, []);

  useEffect(() => {
    inputRef.current?.focus();



    let unlistenFocus: Promise<() => void> | null = null;
    let unlistenPos: Promise<() => void> | null = null;

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      unlistenFocus = listen('tauri://focus', () => {
        fetchClipsRef.current();
        setTimeout(() => inputRef.current?.focus(), 50);
      });

      unlistenPos = listen<{
        flipped: boolean;
        cursorX: number;
        cursorY: number;
        targetX: number;
        targetY: number;
      }>('hud_position_updated', ({ payload }) => {
        setHudAnchor({
          flipped: payload.flipped,
          x: Math.min(342, Math.max(18, payload.cursorX - payload.targetX)),
        });
      });
    }

    return () => {
      if (unlistenFocus) unlistenFocus.then((f) => f());
      if (unlistenPos) unlistenPos.then((f) => f());
    };
  }, []);

  return (
    <div
      className="w-screen h-screen p-0 bg-transparent flex flex-col font-sans select-none overflow-hidden no-drag relative"
      onKeyDownCapture={handleHudKeyDown}
    >
      <div
        aria-hidden="true"
        className={`quick-hud-caret ${hudAnchor.flipped ? 'is-bottom' : 'is-top'}`}
        style={{ left: `${hudAnchor.x}px` }}
      />
      <div className={`quick-hud-shell flex-1 rounded-xl border flex flex-col overflow-hidden no-drag shadow-none ${hudAnchor.flipped ? 'mb-2' : 'mt-2'}`}>
        {/* Header Bar */}
        <div className="quick-hud-header p-2.5 border-b flex items-center space-x-2 no-drag">
          <div className="relative flex-1">
            <Search className="theme-text-muted absolute start-2.5 top-2.5 h-3.5 w-3.5" />
            <input
              ref={inputRef}
              type="text"
              placeholder={translate('component.quickHudWindow.searchClips')}
              value={search}
              onChange={(e) => {
                setPasteError('');
                setSearch(e.target.value);
              }}
              className="theme-input ui-field-radius quick-hud-search w-full border ps-8 pe-3 py-1.5 text-xs font-mono no-drag"
            />
          </div>
          <button
            onClick={() => invoke('toggle_hud_window')}
            className="theme-icon-button p-1 rounded-md border border-transparent transition-colors shrink-0 no-drag"
            title={translate('component.quickHudWindow.hideEsc')}
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Recent Clips 1..9 */}
        <div
          ref={listRef}
          role="listbox"
          aria-label={translate('component.quickHudWindow.recentClips')}
          aria-busy={isPasting}
          className="custom-scrollbar flex-1 overflow-y-auto p-2 space-y-1.5"
        >
          {clips.length === 0 ? (
            <div className="theme-text-subtle flex flex-col items-center justify-center h-48 text-center space-y-1.5 p-4">
              <Sparkles className="w-6 h-6" />
              <p className="theme-text-muted text-xs font-semibold">{translate('component.quickHudWindow.noMatchingClips')}</p>
            </div>
          ) : (
            clips.map((clip, index) => {
              const isSel = index === selectedIndex;
              const previewText = clip.content_type === 'image' && clip.image_base64
                ? clip.text_content ? translate('component.quickHudWindow.ocrText', { text: clip.text_content }) : translate('component.quickHudWindow.screenshotImage')
                : clip.content_type === 'file'
                  ? getClipFileSummary(clip)
                  : clip.text_content || '';
              return (
                <div
                  key={clip.id}
                  data-hud-index={index}
                  role="option"
                  aria-selected={isSel}
                  onPointerDown={() => setSelectedIndex(index)}
                  onClick={() => activateClip(clip)}
                  className={`quick-hud-row p-2.5 rounded-xl border cursor-pointer flex items-center justify-between space-x-3 ${isSel ? 'is-selected shadow-md' : ''}`}
                >
                  <div className="flex items-center space-x-2.5 min-w-0 flex-1">
                    <span
                      className={`quick-hud-index w-5 h-5 rounded-md flex items-center justify-center font-mono text-[0.6875rem] font-extrabold shrink-0 border ${isSel ? 'is-selected shadow' : ''}`}
                    >
                      {index + 1}
                    </span>

                    <div className="min-w-0 flex-1">
                      {clip.content_type === 'image' && clip.image_base64 ? (
                        <div className="flex items-center space-x-2">
                          <SafeRasterImage
                            source={clip.image_base64}
                            alt={translate('component.quickHudWindow.clipPreview')}
                            className="theme-divider h-8 w-12 object-cover rounded border"
                          />
                          <OverflowText text={previewText} className="theme-text-muted text-xs font-mono truncate" />
                        </div>
                      ) : clip.content_type === 'file' ? (
                        <OverflowText as="p" text={previewText} className="text-xs font-mono truncate leading-snug" />
                      ) : (
                        <OverflowText as="p" text={previewText} className="text-xs font-mono truncate leading-snug" />
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Bottom Quick Help Bar */}
        <div className="quick-hud-footer min-h-8 px-3 py-2 border-t flex items-center justify-between gap-3 text-[0.625rem] font-mono">
          {isPasting ? (
            <span className="min-w-0 flex flex-1 items-center gap-1.5">
              <LoaderCircle className="h-3 w-3 shrink-0 animate-spin" />
              <span>{translate('component.quickHudWindow.pasting')}</span>
            </span>
          ) : pasteError ? (
            <span className="theme-danger-text min-w-0 flex flex-1 items-center gap-1.5" title={pasteError}>
              <AlertCircle className="h-3 w-3 shrink-0" />
              <span className="truncate">{pasteError}</span>
            </span>
          ) : (
            <>
              <span>{translate('component.quickHudWindow.value19OrEnterToPaste')}</span>
              <span>{translate('component.quickHudWindow.escToHide')}</span>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
