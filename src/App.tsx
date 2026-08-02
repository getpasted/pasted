import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { enable, disable } from '@tauri-apps/plugin-autostart';
import { ClipItem, Board, FilterRule, SequentialStatus, AppSettings, BlacklistApp, getClipNoteSummary } from './types';
import { Sidebar } from './components/Sidebar';
import { ClipCard } from './components/ClipCard';
import { ClipPreview } from './components/ClipPreview';
import { SequentialQueueBar } from './components/SequentialQueueBar';
import { FilterManager } from './components/FilterManager';
import { SettingsModal } from './components/SettingsModal';
import { BoardModal } from './components/BoardModal';
import { ContextMenu } from './components/ContextMenu';
import { QuickHudWindow } from './components/QuickHudWindow';
import { ActivityLogView } from './components/ActivityLogView';
import { AnalyticsView } from './components/AnalyticsView';
import { HelpView } from './components/HelpView';
import { soundManager } from './utils/sound';
import { Clipboard, AlertTriangle, Edit3, Trash2, Pause, Disc, Square } from 'lucide-react';
import './App.css';

export default function App() {
  const [isHudView, setIsHudView] = useState<boolean>(false);

  useEffect(() => {
    const enableHudMode = () => {
      setIsHudView(true);
      document.documentElement.classList.add('hud-mode');
      document.body.classList.add('hud-mode');
      const root = document.getElementById('root');
      if (root) root.classList.add('hud-mode');
    };

    try {
      const win = getCurrentWindow();
      if (win.label === 'hud' || window.location.search.includes('view=hud')) {
        enableHudMode();
      }
    } catch {
      if (window.location.search.includes('view=hud')) {
        enableHudMode();
      }
    }
  }, []);

  const [allClips, setAllClips] = useState<ClipItem[]>(() => {
    try {
      const cached = localStorage.getItem('pasted_cache_clips');
      return cached ? JSON.parse(cached) : [];
    } catch {
      return [];
    }
  });
  const [clips, setClips] = useState<ClipItem[]>(allClips);
  const [trashedClips, setTrashedClips] = useState<ClipItem[]>([]);
  const [selectedClip, setSelectedClip] = useState<ClipItem | null>(null);
  const [, setSelectedIndex] = useState<number>(0);
  const [totalClipCount, setTotalClipCount] = useState<number>(0);

  const fetchTrashedClips = useCallback(async () => {
    try {
      const res = await invoke<ClipItem[]>('get_trashed_clips');
      setTrashedClips(res);
    } catch (err) {
      console.error('Failed to fetch trashed clips:', err);
    }
  }, []);

  const handleRestoreClip = async (clipId: number) => {
    try {
      await invoke('restore_clip', { id: clipId });
      fetchClips();
      fetchTrashedClips();
    } catch (err) {
      console.error(err);
    }
  };

  const handlePurgeClipPermanently = async (clipId: number) => {
    try {
      await invoke('purge_clip_permanently', { id: clipId });
      fetchTrashedClips();
    } catch (err) {
      console.error(err);
    }
  };

  const handleEmptyTrash = async () => {
    try {
      await invoke('empty_trash');
      fetchTrashedClips();
    } catch (err) {
      console.error(err);
    }
  };

  const [boards, setBoards] = useState<Board[]>(() => {
    try {
      const cached = localStorage.getItem('pasted_cache_boards');
      return cached ? JSON.parse(cached) : [];
    } catch {
      return [];
    }
  });
  const [filters, setFilters] = useState<FilterRule[]>([]);
  const [seqStatus, setSeqStatus] = useState<SequentialStatus | null>(null);

  const [currentTab, setCurrentTab] = useState<string>('all');
  const [selectedBoardId, setSelectedBoardId] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [isBoardModalOpen, setIsBoardModalOpen] = useState<boolean>(false);
  const [editingBoard, setEditingBoard] = useState<Board | null>(null);
  const [isClearConfirmOpen, setIsClearConfirmOpen] = useState<boolean>(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(false);

  const clearConfirmModalRef = useRef<HTMLDivElement>(null);

  // Focus Trap for Clear History Confirmation Modal
  useEffect(() => {
    if (!isClearConfirmOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsClearConfirmOpen(false);
        return;
      }

      if (e.key === 'Tab' && clearConfirmModalRef.current) {
        const focusables = clearConfirmModalRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
        );
        if (focusables.length === 0) return;

        const firstElement = focusables[0];
        const lastElement = focusables[focusables.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === firstElement) {
            e.preventDefault();
            lastElement.focus();
          }
        } else {
          if (document.activeElement === lastElement) {
            e.preventDefault();
            firstElement.focus();
          }
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isClearConfirmOpen]);

  // App Settings State
  const [appSettings, setAppSettings] = useState<AppSettings>({
    textSize: 16,
    enableSounds: true,
    openAtLogin: true,
    dockMenubarIcon: 'auto_hide',
    maxClipSizeMb: 100,
    keepClipCount: 900,
    alwaysPastePlainText: false,
    rowHeight: 'medium',
    iCloudSync: true,
    themeMode: 'system',
    spotlightSync: true,
    enableActivityLog: true,
    activityLogCapacity: 1000,
    enableTrash: true,
    trashCapacityCount: 500,
    hudHotkey: 'Alt+Shift+V',
    seqToggleHotkey: 'Alt+Shift+C',
    seqPopHotkey: 'Alt+Shift+X',
  });

  // Light / Dark / System Theme Switcher Engine
  useEffect(() => {
    const applyTheme = () => {
      const mode = appSettings.themeMode || 'system';
      let isLight = false;
      if (mode === 'light') {
        isLight = true;
      } else if (mode === 'system') {
        isLight = window.matchMedia('(prefers-color-scheme: light)').matches;
      }

      if (isLight) {
        document.documentElement.classList.add('light');
      } else {
        document.documentElement.classList.remove('light');
      }
    };

    applyTheme();

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      if ((appSettings.themeMode || 'system') === 'system') {
        applyTheme();
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [appSettings.themeMode]);

  // Pause & Blacklist Live Monitoring State
  const [isClipboardPaused, setIsClipboardPaused] = useState<boolean>(false);
  const [ignoredAppStatus, setIgnoredAppStatus] = useState<{ app_name: string; timestamp: number } | null>(null);

  const handleToggleClipboardPause = async () => {
    try {
      const paused = await invoke<boolean>('toggle_clipboard_pause');
      setIsClipboardPaused(paused);
    } catch (e) {
      console.error('Failed to toggle clipboard pause:', e);
    }
  };

  const handleToggleCopyQueue = async () => {
    try {
      if (seqStatus?.is_active) {
        await invoke('stop_sequential_paste');
      } else {
        await invoke('start_sequential_paste');
        setCurrentTab('sequential');
        setSelectedBoardId(null);
      }
      fetchSequentialStatus();
    } catch (e) {
      console.error('Failed to toggle copy queue:', e);
    }
  };

  // Blacklist Apps State
  const [blacklistApps, setBlacklistApps] = useState<BlacklistApp[]>(() => {
    try {
      const cached = localStorage.getItem('pasted_cache_blacklist_apps');
      return cached ? JSON.parse(cached) : [
        { id: '1', name: '1Password', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
        { id: '2', name: 'Passwords', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
        { id: '3', name: 'Keychain Access', icon: 'Key', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
        { id: '4', name: 'Bitwarden', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
      ];
    } catch {
      return [
        { id: '1', name: '1Password', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
        { id: '2', name: 'Passwords', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
        { id: '3', name: 'Keychain Access', icon: 'Key', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
        { id: '4', name: 'Bitwarden', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
      ];
    }
  });

  // Sync blacklistApps to SQLite backend and localStorage
  useEffect(() => {
    try {
      localStorage.setItem('pasted_cache_blacklist_apps', JSON.stringify(blacklistApps));
    } catch {}
    invoke('save_app_setting', {
      key: 'blacklistApps',
      value: JSON.stringify(blacklistApps),
    }).catch(console.error);
  }, [blacklistApps]);

  // Context Menu State
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    clip: ClipItem;
  } | null>(null);

  // Board Context Menu State
  const [boardContextMenu, setBoardContextMenu] = useState<{
    x: number;
    y: number;
    board: Board;
  } | null>(null);

  // Resizable Column Widths (stored in localStorage with min/max bounds)
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    const saved = localStorage.getItem('pasted_sidebar_width');
    return saved ? parseInt(saved, 10) : 240;
  });
  const [clipsListWidth, setClipsListWidth] = useState<number>(() => {
    const saved = localStorage.getItem('pasted_list_width');
    return saved ? parseInt(saved, 10) : 340;
  });

  const [isResizingSidebar, setIsResizingSidebar] = useState(false);
  const [isResizingList, setIsResizingList] = useState(false);

  const handleSidebarMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = isSidebarCollapsed ? 100 : sidebarWidth;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const delta = moveEvent.clientX - startX;
      const newWidth = Math.min(Math.max(startWidth + delta, 180), 360);
      setSidebarWidth(newWidth);
      localStorage.setItem('pasted_sidebar_width', newWidth.toString());
    };

    const handleMouseUp = () => {
      setIsResizingSidebar(false);
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };

    setIsResizingSidebar(true);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const handleListMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = clipsListWidth;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const delta = moveEvent.clientX - startX;
      const newWidth = Math.min(Math.max(startWidth + delta, 280), 520);
      setClipsListWidth(newWidth);
      localStorage.setItem('pasted_list_width', newWidth.toString());
    };

    const handleMouseUp = () => {
      setIsResizingList(false);
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };

    setIsResizingList(true);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  useEffect(() => {
    if (!boardContextMenu) return;
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && target.closest('.board-context-menu')) return;
      setBoardContextMenu(null);
    };
    window.addEventListener('mousedown', handleClickOutside);
    return () => window.removeEventListener('mousedown', handleClickOutside);
  }, [boardContextMenu]);

  // Load saved settings from SQLite database on mount
  useEffect(() => {
    invoke<Record<string, string>>('get_all_app_settings')
      .then((saved) => {
        if (saved && Object.keys(saved).length > 0) {
          setAppSettings((prev) => {
            const next = { ...prev };
            if (saved.textSize) next.textSize = Number(saved.textSize);
            if (saved.enableSounds !== undefined) next.enableSounds = saved.enableSounds === 'true';
            if (saved.openAtLogin !== undefined) next.openAtLogin = saved.openAtLogin === 'true';
            if (saved.dockMenubarIcon) next.dockMenubarIcon = saved.dockMenubarIcon as any;
            if (saved.maxClipSizeMb) next.maxClipSizeMb = Number(saved.maxClipSizeMb);
            if (saved.keepClipCount) next.keepClipCount = Number(saved.keepClipCount);
            if (saved.alwaysPastePlainText !== undefined) next.alwaysPastePlainText = saved.alwaysPastePlainText === 'true';
            if (saved.rowHeight) next.rowHeight = saved.rowHeight as any;
            if (saved.iCloudSync !== undefined) next.iCloudSync = saved.iCloudSync === 'true';
            if (saved.hudHotkey !== undefined) next.hudHotkey = saved.hudHotkey;
            if (saved.seqToggleHotkey !== undefined) next.seqToggleHotkey = saved.seqToggleHotkey;
            if (saved.seqPopHotkey !== undefined) next.seqPopHotkey = saved.seqPopHotkey;
            if (saved.pasteLastFilterHotkey !== undefined) next.pasteLastFilterHotkey = saved.pasteLastFilterHotkey;
            if (saved.openFilterWindowHotkey !== undefined) next.openFilterWindowHotkey = saved.openFilterWindowHotkey;
            if (saved.openMainWindowHotkey !== undefined) next.openMainWindowHotkey = saved.openMainWindowHotkey;
            for (let i = 1; i <= 9; i++) {
              const k = `pasteClip${i}Hotkey`;
              if (saved[k] !== undefined) (next as any)[k] = saved[k];
            }
            return next;
          });
        }
      })
      .catch(console.error);
  }, []);

  // Apply Font Size dynamically
  useEffect(() => {
    document.documentElement.style.fontSize = `${appSettings.textSize}px`;
  }, [appSettings.textSize]);

  // Disable WebKit default right-click context menu (Reload/Inspect) app-wide
  useEffect(() => {
    const handleGlobalContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };
    window.addEventListener('contextmenu', handleGlobalContextMenu);
    return () => window.removeEventListener('contextmenu', handleGlobalContextMenu);
  }, []);

  // Sync Autostart setting with OS
  useEffect(() => {
    if (appSettings.openAtLogin) {
      enable().catch(console.error);
    } else {
      disable().catch(console.error);
    }
  }, [appSettings.openAtLogin]);

  // Enforce Clip Retention Count
  useEffect(() => {
    invoke('enforce_clip_retention', { keepCount: appSettings.keepClipCount }).catch(console.error);
  }, [appSettings.keepClipCount]);

  // Sync Dock & Menubar visibility setting with macOS immediately
  useEffect(() => {
    const showDock = appSettings.dockMenubarIcon === 'both';
    invoke('set_dock_visibility', { showDock }).catch(console.error);
  }, [appSettings.dockMenubarIcon]);

  // Fetch Total Clip Count
  const fetchTotalClipCount = useCallback(async () => {
    try {
      const count = await invoke<number>('get_total_clip_count');
      setTotalClipCount(count);
    } catch (e) {
      console.error('Failed to fetch total count:', e);
    }
  }, []);

  // Fetch Clips from Backend SQLite (Only runs when clips change)
  const fetchClips = useCallback(async () => {
    try {
      const fullRes = await invoke<ClipItem[]>('get_clips', {
        searchQuery: null,
        boardId: null,
        onlyPinned: false,
      });
      setAllClips(fullRes);
      setClips(fullRes);
      fetchTotalClipCount();
      try {
        localStorage.setItem('pasted_cache_clips', JSON.stringify(fullRes.slice(0, 50)));
      } catch {
        // Ignore cache limits
      }
    } catch (e) {
      console.error('Failed to fetch clips:', e);
    }
  }, [fetchTotalClipCount]);

  // Instantaneous (0ms) In-Memory View Filtering Memo
  const displayedClips = useMemo(() => {
    if (currentTab === 'sequential') {
      if (seqStatus && seqStatus.queue && seqStatus.queue.length > 0) {
        return seqStatus.queue.map((text, idx): ClipItem => ({
          id: 999000 + idx,
          content_type: 'text',
          text_content: text,
          html_content: null,
          image_base64: null,
          content_hash: `queue_${idx}`,
          source_app: `Queue Position #${idx + 1}`,
          board_id: null,
          is_pinned: false,
          note: null,
          created_at: new Date().toISOString(),
        }));
      }
      return [];
    }

    let list = allClips;

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      list = list.filter(
        (c) =>
          c.text_content?.toLowerCase().includes(q) ||
          c.source_app?.toLowerCase().includes(q) ||
          getClipNoteSummary(c.note).toLowerCase().includes(q) ||
          c.content_type?.toLowerCase().includes(q)
      );
    }

    if (currentTab === 'trash') {
      list = trashedClips;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase().trim();
        list = list.filter(
          (c) =>
            c.text_content?.toLowerCase().includes(q) ||
            c.source_app?.toLowerCase().includes(q) ||
            getClipNoteSummary(c.note).toLowerCase().includes(q) ||
            c.content_type?.toLowerCase().includes(q)
        );
      }
      return list;
    }

    if (currentTab === 'board' && selectedBoardId !== null) {
      const activeBin = boards.find((b) => b.id === selectedBoardId);
      if (activeBin && activeBin.smart_rule) {
        try {
          const parsed = JSON.parse(activeBin.smart_rule);
          const matchMode = parsed.match || 'any';
          const conds: Array<{ type: string; operator?: string; value: string }> =
            parsed.conditions || (parsed.type ? [{ type: parsed.type, value: parsed.value }] : []);

          if (conds.length > 0) {
            list = list.filter((clip) => {
              if (clip.board_id === selectedBoardId) return true;

              const checkCond = (cond: { type: string; operator?: string; value: string }) => {
                const val = (cond.value || '').toLowerCase().trim();
                if (!val) return false;
                if (cond.type === 'content_type') {
                  return clip.content_type?.toLowerCase() === val;
                }
                if (cond.type === 'source_app') {
                  return clip.source_app?.toLowerCase().includes(val);
                }
                if (cond.type === 'contains') {
                  return clip.text_content?.toLowerCase().includes(val);
                }
                return false;
              };

              if (matchMode === 'all') {
                return conds.every(checkCond);
              } else {
                return conds.some(checkCond);
              }
            });
          } else {
            list = list.filter((c) => c.board_id === selectedBoardId);
          }
        } catch {
          list = list.filter((c) => c.board_id === selectedBoardId);
        }
      } else {
        list = list.filter((c) => c.board_id === selectedBoardId);
      }
    }

    if (currentTab === 'pinned') {
      list = list.filter((c) => c.is_pinned);
    } else if (currentTab === 'protected') {
      list = list.filter((c) => c.is_protected);
    } else if (currentTab === 'notes') {
      list = list.filter((c) => c.note && c.note.trim().length > 0);
    }

    return list;
  }, [allClips, trashedClips, searchQuery, currentTab, selectedBoardId, seqStatus, boards]);

  // Keep selected clip valid on view switch
  useEffect(() => {
    if (displayedClips.length > 0) {
      setSelectedClip((prev) => {
        if (prev) {
          const found = displayedClips.find((c) => c.id === prev.id);
          return found || displayedClips[0];
        }
        return displayedClips[0];
      });
    } else {
      setSelectedClip(null);
    }
  }, [displayedClips]);

  // Fetch Boards
  const fetchBoards = useCallback(async () => {
    try {
      const res = await invoke<Board[]>('get_boards');
      setBoards(res);
      try {
        localStorage.setItem('pasted_cache_boards', JSON.stringify(res));
      } catch {
        // Ignore cache limits
      }
    } catch (e) {
      console.error('Failed to fetch boards:', e);
    }
  }, []);

  // Fetch Filters
  const fetchFilters = useCallback(async () => {
    try {
      const res = await invoke<FilterRule[]>('get_filters');
      setFilters(res);
    } catch (e) {
      console.error('Failed to fetch filters:', e);
    }
  }, []);

  // Fetch Sequential Status
  const fetchSequentialStatus = useCallback(async () => {
    try {
      const res = await invoke<SequentialStatus>('get_sequential_status');
      setSeqStatus(res);
    } catch (e) {
      console.error('Failed to fetch sequential status:', e);
    }
  }, []);

  useEffect(() => {
    Promise.all([
      fetchClips(),
      fetchBoards(),
      fetchFilters(),
      fetchSequentialStatus(),
      fetchTrashedClips(),
      invoke<boolean>('is_clipboard_paused')
        .then((paused) => setIsClipboardPaused(paused))
        .catch((e) => console.error(e)),
    ]).catch(console.error);
  }, [fetchClips, fetchBoards, fetchFilters, fetchSequentialStatus, fetchTrashedClips]);

  // Event Listeners for Tauri Rust backend
  useEffect(() => {
    const unlistenClip = listen<ClipItem>('clip-added', () => {
      fetchClips();
      soundManager.playCopySound(appSettings.enableSounds);
    });

    const unlistenSeq = listen<SequentialStatus>('sequential-updated', (evt) => {
      setSeqStatus(evt.payload);
    });

    const unlistenBlacklist = listen<{ app_name: string }>('blacklist-clip-ignored', (evt) => {
      setIgnoredAppStatus({ app_name: evt.payload.app_name, timestamp: Date.now() });
      setTimeout(() => {
        setIgnoredAppStatus(null);
      }, 4000);
    });

    const unlistenPause = listen<{ is_paused: boolean; auto_paused_by: string | null }>(
      'clipboard-pause-changed',
      (evt) => {
        setIsClipboardPaused(evt.payload.is_paused);
      }
    );

    return () => {
      unlistenClip.then((f) => f());
      unlistenSeq.then((f) => f());
      unlistenBlacklist.then((f) => f());
      unlistenPause.then((f) => f());
    };
  }, [fetchClips, appSettings.enableSounds]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === '\\') {
        e.preventDefault();
        setIsSidebarCollapsed((prev) => !prev);
        return;
      }

      if (['INPUT', 'TEXTAREA', 'SELECT'].includes((e.target as HTMLElement).tagName)) {
        return;
      }

      if (clips.length === 0) return;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => {
          const next = Math.min(prev + 1, clips.length - 1);
          setSelectedClip(clips[next]);
          return next;
        });
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => {
          const next = Math.max(prev - 1, 0);
          setSelectedClip(clips[next]);
          return next;
        });
      } else if (e.key === 'Enter' && selectedClip) {
        e.preventDefault();
        handleCopyClip(selectedClip);
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedClip) {
          e.preventDefault();
          handleDeleteClip(selectedClip.id);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [clips, selectedClip]);

  const [deletingClipIds, setDeletingClipIds] = useState<Set<number>>(new Set());

  const handleTogglePin = async (id: number) => {
    try {
      await invoke('toggle_pin_clip', { id });
      fetchClips();
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleProtected = async (id: number) => {
    try {
      const isNowProtected = await invoke<boolean>('toggle_clip_protected', { clipId: id });
      setAllClips((prev) =>
        prev.map((c) => (c.id === id ? { ...c, is_protected: isNowProtected } : c))
      );
      if (selectedClip && selectedClip.id === id) {
        setSelectedClip((prev) => (prev ? { ...prev, is_protected: isNowProtected } : null));
      }
    } catch (e) {
      console.error('Failed to toggle protected state:', e);
    }
  };

  const handleDeleteClip = async (id: number, forcePermanent = false) => {
    setDeletingClipIds((prev) => new Set(prev).add(id));
    setTimeout(async () => {
      // Optimistically remove from local state so item never flickers back
      setClips((prev) => prev.filter((c) => c.id !== id));
      setAllClips((prev) => prev.filter((c) => c.id !== id));
      if (selectedClip?.id === id) {
        setSelectedClip(null);
      }
      setDeletingClipIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });

      try {
        if (forcePermanent || appSettings.enableTrash === false) {
          await invoke('purge_clip_permanently', { id });
        } else {
          await invoke('delete_clip', { id });
        }
        setTotalClipCount((prev) => Math.max(0, prev - 1));
        fetchClips();
        fetchTrashedClips();
      } catch (e) {
        console.error(e);
        fetchClips();
        fetchTrashedClips();
      }
    }, 200);
  };

  const handleCopyClip = async (clip: ClipItem) => {
    try {
      let textToCopy = clip.text_content;

      // If alwaysPastePlainText is true, strip any HTML tags or rich formatting
      if (appSettings.alwaysPastePlainText && textToCopy) {
        textToCopy = textToCopy.replace(/<[^>]*>/g, '');
      }

      await invoke('copy_clip_to_system', {
        text: textToCopy,
        imageBase64: appSettings.alwaysPastePlainText ? null : clip.image_base64,
      });

      soundManager.playCopySound(appSettings.enableSounds);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAssignBoard = async (clipId: number, boardId: number | null) => {
    try {
      await invoke('assign_clip_board', { clipId, boardId });
      fetchClips();
    } catch (e) {
      console.error(e);
    }
  };

  const handleApplyFilterToClip = async (clip: ClipItem, filter: FilterRule) => {
    if (!clip.text_content) return;
    try {
      const res = await invoke<string>('transform_text', {
        input: clip.text_content,
        filterType: filter.filter_type,
        config: filter.config,
      });
      await invoke('copy_clip_to_system', { text: res, imageBase64: null });
      soundManager.playPasteSound(appSettings.enableSounds);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAddToSequentialStack = async (text: string | null) => {
    if (!text) return;
    try {
      await invoke('push_sequential_item', { item: text });
      soundManager.playStackSound(appSettings.enableSounds);
      fetchSequentialStatus();
    } catch (e) {
      console.error(e);
    }
  };

  const handlePromptAddNote = async (clip: ClipItem) => {
    const existingNote = clip.note || '';
    const newNote = window.prompt('Enter note/annotation for clip:', existingNote);
    if (newNote !== null) {
      try {
        await invoke('update_clip_note', {
          clipId: clip.id,
          note: newNote.trim() || null,
        });
        fetchClips();
      } catch (e) {
        console.error(e);
      }
    }
  };

  const handleUpdateClipNoteLocally = useCallback((clipId: number, newNote: string | null) => {
    setAllClips((prev) =>
      prev.map((c) => (c.id === clipId ? { ...c, note: newNote } : c))
    );
    setSelectedClip((prev) => (prev && prev.id === clipId ? { ...prev, note: newNote } : prev));
  }, []);

  const handleDeleteNoteFromClip = async (clipId: number) => {
    handleUpdateClipNoteLocally(clipId, null);
    try {
      await invoke('update_clip_note', {
        clipId,
        note: null,
      });
    } catch (e) {
      console.error(e);
    }
  };

  const handleClearHistory = async () => {
    try {
      await invoke('clear_history');
      setIsClearConfirmOpen(false);
      fetchClips();
    } catch (e) {
      console.error(e);
    }
  };

  const settingsSaveTimerRef = useRef<{ [key: string]: ReturnType<typeof setTimeout> }>({});

  const handleUpdateSettings = (newSettings: Partial<AppSettings>) => {
    setAppSettings((prev) => {
      const updated = { ...prev, ...newSettings };
      // Debounce disk writes to keep UI slider interactions 100% instant
      for (const [k, v] of Object.entries(newSettings)) {
        if (settingsSaveTimerRef.current[k]) {
          clearTimeout(settingsSaveTimerRef.current[k]);
        }
        settingsSaveTimerRef.current[k] = setTimeout(() => {
          invoke('save_app_setting', { key: k, value: String(v) }).catch(console.error);
        }, 250);
      }
      return updated;
    });
  };

  const handleAddBlacklistApp = (appName: string) => {
    setBlacklistApps((prev) => [
      ...prev,
      {
        id: String(Date.now()),
        name: appName,
        icon: 'Lock',
        ignoreText: true,
        ignoreImages: true,
        ignoreShortcuts: false,
      },
    ]);
  };

  const handleRemoveBlacklistApp = (appId: string) => {
    setBlacklistApps((prev) => prev.filter((a) => a.id !== appId));
  };

  const handleToggleBlacklistRule = (
    appId: string,
    rule: 'ignoreText' | 'ignoreImages' | 'ignoreShortcuts'
  ) => {
    setBlacklistApps((prev) =>
      prev.map((a) => (a.id === appId ? { ...a, [rule]: !a[rule] } : a))
    );
  };

  if (isHudView) {
    return <QuickHudWindow />;
  }

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#171717] text-gray-100 font-sans">
      {/* Left macOS Sidebar */}
      <Sidebar
        currentTab={currentTab}
        setCurrentTab={setCurrentTab}
        selectedBoardId={selectedBoardId}
        setSelectedBoardId={setSelectedBoardId}
        boards={boards}
        onRefreshBoards={fetchBoards}
        onOpenNewBoardModal={() => {
          setEditingBoard(null);
          setIsBoardModalOpen(true);
        }}
        onEditBoard={(board) => {
          setEditingBoard(board);
          setIsBoardModalOpen(true);
        }}
        onBoardContextMenu={(x, y, board) => setBoardContextMenu({ x, y, board })}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        seqStatus={seqStatus}
        onClearHistory={() => setIsClearConfirmOpen(true)}
        pinnedCount={allClips.filter((c) => c.is_pinned).length}
        protectedCount={allClips.filter((c) => c.is_protected).length}
        notesCount={allClips.filter((c) => c.note && c.note.trim().length > 0).length}
        trashedCount={trashedClips.length}
        totalClipCount={totalClipCount}
        isCollapsed={isSidebarCollapsed}
        setIsCollapsed={setIsSidebarCollapsed}
        sidebarWidth={sidebarWidth}
      />

      {/* Sidebar Resizer Handle (Only active when sidebar is expanded) */}
      {!isSidebarCollapsed && (
        <div
          onMouseDown={handleSidebarMouseDown}
          className="relative w-[1px] h-screen cursor-col-resize z-30 shrink-0 select-none group"
          title="Drag to resize sidebar width"
        >
          <div
            className={`w-[1px] h-full transition-colors ${
              isResizingSidebar ? 'bg-[#0a84ff]' : 'bg-[#2d2d2d] group-hover:bg-[#0a84ff]'
            }`}
          />
          <div className="absolute inset-y-0 -left-1 -right-1 z-40 cursor-col-resize" />
        </div>
      )}

      {/* Main Content Area */}
      {currentTab === 'filters' || currentTab === 'operations' ? (
        <FilterManager filters={filters} onRefreshFilters={fetchFilters} />
      ) : currentTab === 'activity' ? (
        <ActivityLogView />
      ) : currentTab === 'analytics' ? (
        <AnalyticsView />
      ) : currentTab === 'help' ? (
        <HelpView />
      ) : currentTab === 'settings' ? (
        <SettingsModal
          settings={appSettings}
          onUpdateSettings={handleUpdateSettings}
          blacklistApps={blacklistApps}
          onAddBlacklistApp={handleAddBlacklistApp}
          onRemoveBlacklistApp={handleRemoveBlacklistApp}
          onToggleBlacklistRule={handleToggleBlacklistRule}
          filters={filters}
          onRefreshFilters={fetchFilters}
          boards={boards}
          onRefreshBoards={fetchBoards}
          onClearHistory={() => setIsClearConfirmOpen(true)}
          onResetColumnWidths={() => {
            setSidebarWidth(240);
            setClipsListWidth(340);
            localStorage.removeItem('pasted_sidebar_width');
            localStorage.removeItem('pasted_list_width');
          }}
        />
      ) : (
        <div className="flex-1 h-screen flex overflow-hidden">
          {/* Middle Clips List Panel */}
          <div
            style={{ width: `${clipsListWidth}px` }}
            className="shrink-0 col-list h-screen flex flex-col bg-[#171717] overflow-hidden"
          >
            {/* Finder Header Title Bar */}
            <div
              data-tauri-drag-region
              className="h-[60px] border-b border-[#2b2b2b] bg-[#171717]/80 backdrop-blur-md px-3 flex items-center justify-between col-list-header cursor-default titlebar-drag-handle shrink-0"
            >
              <div data-tauri-drag-region className="flex items-center space-x-2 titlebar-drag-handle min-w-0 flex-1 mr-2">
                <Clipboard className="w-4 h-4 text-gray-300 titlebar-drag-handle shrink-0" />
                <h2 data-tauri-drag-region className="text-xs font-bold text-gray-200 uppercase tracking-wider titlebar-drag-handle truncate">
                  {currentTab === 'pinned'
                    ? 'Pinned'
                    : currentTab === 'protected'
                    ? 'Protected'
                    : currentTab === 'notes'
                    ? 'Noted'
                    : currentTab === 'sequential'
                    ? 'Queue'
                    : currentTab === 'trash'
                    ? 'Trashed'
                    : selectedBoardId
                    ? boards.find((b) => b.id === selectedBoardId)?.name || 'Board'
                    : 'All History'}
                </h2>
              </div>

              {/* Global Controls & Status Badges */}
              <div className="flex items-center space-x-1.5 shrink-0">
                {ignoredAppStatus && (
                  <span className="text-[10px] px-2 py-0.5 rounded bg-red-950/80 border border-red-800/60 text-red-300 font-mono flex items-center animate-in fade-in">
                    Ignored: {ignoredAppStatus.app_name}
                  </span>
                )}

                {currentTab === 'trash' && (
                  <button
                    onClick={handleEmptyTrash}
                    disabled={trashedClips.length === 0}
                    className="px-2 py-1 rounded-lg bg-rose-600/20 hover:bg-rose-600/30 text-rose-300 border border-rose-500/30 text-xs font-semibold disabled:opacity-40 transition-all cursor-pointer flex items-center space-x-1"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Empty Trash</span>
                  </button>
                )}

                {/* Pause History Toggle Button */}
                <button
                  onClick={handleToggleClipboardPause}
                  className={`w-7 h-7 flex items-center justify-center rounded-lg transition-all cursor-pointer ${
                    isClipboardPaused
                      ? 'bg-amber-500/20 text-amber-400 border border-amber-500/40 shadow-sm'
                      : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800 border border-transparent'
                  }`}
                  title={isClipboardPaused ? 'Resume History Recording' : 'Pause History Recording (for sensitive items/passwords)'}
                >
                  <Pause
                    className={`w-4 h-4 ${isClipboardPaused ? 'fill-amber-400 text-amber-400 animate-pulse' : 'text-gray-400'}`}
                    strokeWidth={2.5}
                  />
                </button>

                {/* Copy Queue Record/Stop Toggle Button */}
                <button
                  onClick={handleToggleCopyQueue}
                  className={`w-7 h-7 flex items-center justify-center rounded-lg transition-all cursor-pointer ${
                    seqStatus?.is_active
                      ? 'bg-purple-500/20 text-purple-300 border border-purple-500/40 shadow-sm'
                      : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800 border border-transparent'
                  }`}
                  title={seqStatus?.is_active ? `Stop Queue Recording (${seqStatus.queue.length} items queued)` : 'Start Queue Recording'}
                >
                  {seqStatus?.is_active ? (
                    <Square className="w-3.5 h-3.5 fill-purple-400 text-purple-400 animate-pulse" strokeWidth={2.5} />
                  ) : (
                    <Disc className="w-4 h-4 text-gray-400 hover:text-purple-400 transition-colors" strokeWidth={2.5} />
                  )}
                </button>
              </div>
            </div>

            {/* Sequential Paste Top Header Banner if active */}
            {currentTab === 'sequential' && (
              <div className="p-3 bg-purple-950/30 border-b border-purple-500/30">
                <SequentialQueueBar
                  status={seqStatus}
                  onRefresh={fetchSequentialStatus}
                />
              </div>
            )}

            {/* Clips List Content */}
            <div className="flex-1 overflow-y-auto pl-3 pr-3 py-3 space-y-2.5 custom-scrollbar">
              {displayedClips.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-center p-6 text-gray-500 select-none">
                  <Clipboard className="w-10 h-10 mb-3 opacity-30 stroke-1" />
                  <p className="text-xs font-medium text-gray-400">No clips found</p>
                  <p className="text-[11px] text-gray-600 mt-1">
                    {searchQuery ? 'Try matching another search term' : 'Copied items will automatically show up here'}
                  </p>
                </div>
              ) : (
                displayedClips.map((clip, index) => {
                  const queuedItems = seqStatus?.queue || [];
                  const qIdx = clip.text_content ? queuedItems.indexOf(clip.text_content) : -1;
                  const queueIndex = qIdx !== -1 ? qIdx + 1 : undefined;

                  return (
                    <ClipCard
                      key={clip.id}
                      clip={clip}
                      isSelected={selectedClip?.id === clip.id}
                      isDeleting={deletingClipIds.has(clip.id)}
                      isTrashMode={currentTab === 'trash'}
                      isQueueMode={currentTab === 'sequential'}
                      queueIndex={queueIndex}
                      rowHeight={appSettings.rowHeight}
                      onSelect={() => {
                        setSelectedClip(clip);
                        setSelectedIndex(index);
                      }}
                      onPin={() => handleTogglePin(clip.id)}
                      onToggleProtected={() => handleToggleProtected(clip.id)}
                      onDelete={(e) => handleDeleteClip(clip.id, e?.altKey)}
                      onRestore={() => handleRestoreClip(clip.id)}
                      onPurgePermanently={() => handlePurgeClipPermanently(clip.id)}
                      onRemoveFromQueue={() => {
                        if (qIdx !== -1) {
                          invoke('remove_sequential_item_by_index', { index: qIdx }).then(fetchSequentialStatus);
                        }
                      }}
                      onPasteQueueItem={() => {
                        if (qIdx === 0) {
                          invoke('pop_sequential_paste').then(fetchSequentialStatus);
                        } else if (qIdx !== -1) {
                          invoke('remove_sequential_item_by_index', { index: qIdx }).then(fetchSequentialStatus);
                        }
                      }}
                      onCopy={() => handleCopyClip(clip)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setContextMenu({
                          x: e.clientX,
                          y: e.clientY,
                          clip,
                        });
                      }}
                    />
                  );
                })
              )}
            </div>
          </div>

          {/* List Resizer Handle (Exact 1px visual border line with grab target extending to right) */}
          <div
            onMouseDown={handleListMouseDown}
            className="relative w-[1px] h-screen cursor-col-resize z-20 shrink-0 select-none group"
            title="Drag to resize clips list width"
          >
            <div
              className={`w-[1px] h-full transition-colors ${
                isResizingList ? 'bg-[#0a84ff]' : 'bg-[#2b2b2b] group-hover:bg-[#0a84ff]'
              }`}
            />
            <div className="absolute inset-y-0 left-0 -right-2 z-20 cursor-col-resize" />
          </div>

          {/* Right Detail Preview Panel */}
          <ClipPreview
            clip={selectedClip}
            boards={boards}
            filters={filters}
            onUpdateClip={fetchClips}
            onDeleteClip={handleDeleteClip}
            onUpdateClipNote={handleUpdateClipNoteLocally}
          />
        </div>
      )}

      {/* Right Click Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          clip={contextMenu.clip}
          boards={boards}
          filters={filters}
          onClose={() => setContextMenu(null)}
          onCopy={() => handleCopyClip(contextMenu.clip)}
          onAssignBoard={(boardId) => handleAssignBoard(contextMenu.clip.id, boardId)}
          onApplyFilter={(filter) => handleApplyFilterToClip(contextMenu.clip, filter)}
          onAddNote={() => handlePromptAddNote(contextMenu.clip)}
          onDeleteNote={() => handleDeleteNoteFromClip(contextMenu.clip.id)}
          onAddToStack={() => handleAddToSequentialStack(contextMenu.clip.text_content)}
          onTogglePin={() => handleTogglePin(contextMenu.clip.id)}
          onToggleProtected={() => handleToggleProtected(contextMenu.clip.id)}
          onDelete={(e) => handleDeleteClip(contextMenu.clip.id, e?.altKey)}
        />
      )}

      {/* Root-Level macOS Right-Click Context Menu for Custom Boards */}
      {boardContextMenu && (
        <div
          style={{
            top: Math.min(boardContextMenu.y, window.innerHeight - 100),
            left: Math.min(boardContextMenu.x, window.innerWidth - 180),
          }}
          className="board-context-menu fixed z-[9999] min-w-[170px] glass-hud rounded-xl p-1.5 shadow-2xl text-xs font-medium space-y-0.5 animate-in fade-in zoom-in-95 duration-100"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            onClick={() => {
              const b = boardContextMenu.board;
              setBoardContextMenu(null);
              setEditingBoard(b);
              setIsBoardModalOpen(true);
            }}
            className="w-full flex items-center space-x-2 px-2.5 py-1.5 rounded-md hover:bg-blue-600 hover:text-white transition-colors cursor-pointer"
          >
            <Edit3 className="w-3.5 h-3.5" />
            <span>Edit Bin...</span>
          </button>
          <div className="border-t border-white/10 my-1" />
          <button
            type="button"
            onClick={async (e) => {
              e.stopPropagation();
              const b = boardContextMenu.board;
              setBoardContextMenu(null);
              if (confirm(`Delete bin "${b.name}"?`)) {
                try {
                  await invoke('delete_board', { id: b.id });
                  fetchBoards();
                  if (selectedBoardId === b.id) {
                    setCurrentTab('all');
                    setSelectedBoardId(null);
                  }
                } catch (err) {
                  console.error(err);
                }
              }
            }}
            className="w-full flex items-center space-x-2 px-2.5 py-1.5 rounded-md text-red-400 hover:bg-red-600 hover:text-white transition-colors cursor-pointer"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Delete Bin</span>
          </button>
        </div>
      )}

      {/* Custom Board Creator / Editor Modal */}
      <BoardModal
        key={editingBoard ? `edit-${editingBoard.id}` : 'new-bin'}
        isOpen={isBoardModalOpen}
        editingBoard={editingBoard}
        onClose={() => {
          setIsBoardModalOpen(false);
          setEditingBoard(null);
        }}
        onRefreshBoards={fetchBoards}
      />

      {/* Clear History Confirmation Modal */}
      {isClearConfirmOpen && (
        <div ref={clearConfirmModalRef} className="fixed inset-0 bg-black/75 backdrop-blur-md z-50 flex items-center justify-center p-4 select-none">
          <div className="bg-[#212121] w-full max-w-md rounded-2xl p-6 space-y-4 border border-red-500/40 shadow-2xl text-gray-100 font-sans">
            <div className="flex items-center space-x-3 text-red-400">
              <div className="p-2.5 rounded-xl bg-red-500/20 border border-red-500/30">
                <AlertTriangle className="w-6 h-6" />
              </div>
              <div>
                <h3 className="text-base font-bold text-gray-100">Clear Clipboard History?</h3>
                <p className="text-xs text-gray-400">This action cannot be undone.</p>
              </div>
            </div>

            <p className="text-xs text-gray-300 leading-relaxed bg-[#191b22] p-3 rounded-xl border border-gray-700/70">
              Are you sure you want to delete all unpinned clipboard history? Pinned clips and custom bin definitions will be safely preserved.
            </p>

            <div className="flex justify-end space-x-3 pt-2">
              <button
                onClick={() => setIsClearConfirmOpen(false)}
                className="px-4 py-2 rounded-xl bg-[#343744] hover:bg-[#3d4150] text-gray-200 text-xs font-semibold transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleClearHistory}
                className="px-4 py-2 rounded-xl bg-red-600 hover:bg-red-500 text-white text-xs font-semibold shadow-lg shadow-red-600/30 transition-all hover:scale-105 active:scale-95"
              >
                Clear History
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
