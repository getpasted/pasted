import React, { useState, useEffect } from 'react';
import { FilterRule } from '../types';
import { Sliders, Trash2, Code2, Edit3, Sparkles, Copy, Play, Download, Wrench } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { FilterEditorModal } from './FilterEditorModal';
import { OperationsManager } from './OperationsManager';
import { HotkeyRecorder } from './HotkeyRecorder';
import { soundManager } from '../utils/sound';

interface FilterManagerProps {
  filters: FilterRule[];
  onRefreshFilters: () => void;
}

export const FilterManager: React.FC<FilterManagerProps> = ({ filters, onRefreshFilters }) => {
  const [activeSubTab, setActiveSubTab] = useState<'pipelines' | 'operations'>('pipelines');
  const [activeCategory, setActiveCategory] = useState<string>('All');

  const FILTER_CATEGORIES = [
    'All',
    'Cleaners & Sanitizers',
    'Case Transformations',
    'Smart Formatting',
    'Data Extraction',
    'Line Operations',
    'Structure & Formatting',
    'Encodings & Decodings',
    'Advanced & Shell Scripts',
  ];
  const [selectedFilterForEdit, setSelectedFilterForEdit] = useState<FilterRule | null>(null);
  const [isEditorModalOpen, setIsEditorModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello Pasted User! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');
  const [filterContextMenu, setFilterContextMenu] = useState<{ x: number; y: number; filter: FilterRule } | null>(null);

  useEffect(() => {
    const handleClick = () => setFilterContextMenu(null);
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

  const handleOpenCreateModal = () => {
    setSelectedFilterForEdit(null);
    setIsEditorModalOpen(true);
  };

  const handleOpenEditModal = (filter: FilterRule) => {
    setSelectedFilterForEdit(filter);
    setIsEditorModalOpen(true);
  };

  const handleDuplicateFilter = async (filter: FilterRule) => {
    try {
      await invoke('create_filter', {
        name: `${filter.name} (Copy)`,
        filterType: filter.filter_type,
        config: filter.config,
      });
      soundManager.playCopySound(true);
      onRefreshFilters();
    } catch (e) {
      console.error(e);
    }
  };

  const handleExportFilter = async (filter: FilterRule) => {
    try {
      const exportJson = JSON.stringify(filter, null, 2);
      await invoke('copy_clip_to_system', { text: exportJson, imageBase64: null });
      soundManager.playCopySound(true);
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteFilter = async (id: number) => {
    try {
      await invoke('delete_filter', { id });
      onRefreshFilters();
    } catch (e) {
      console.error(e);
    }
  };

  const handleTestTransformation = async (filterType: string, config: string | null) => {
    try {
      const res = await invoke<string>('transform_text', {
        input: testText,
        filterType,
        config,
      });
      setTestResult(res);
    } catch (e) {
      console.error(e);
    }
  };

  const [operationsCount, setOperationsCount] = useState<number>(0);

  const fetchOpCount = () => {
    invoke<any[]>('get_operations')
      .then((ops) => setOperationsCount(ops.length))
      .catch(console.error);
  };

  useEffect(() => {
    fetchOpCount();
  }, [activeSubTab]);

  const openCreateOpRef = React.useRef<(() => void) | null>(null);

  return (
    <div className="tools-page filters-page flex-1 h-screen flex flex-col overflow-hidden bg-[#171717] select-none filter-manager-wrapper">
      {/* 60px Native Titlebar Header Section with Full-Height Square Section Tabs */}
      <div
        data-tauri-drag-region
        className="h-[60px] pl-0 pr-6 border-b border-[#2b2b2b] bg-[#171717]/95 backdrop-blur-md flex items-center justify-between shrink-0 titlebar-drag-handle cursor-default"
      >
        {/* Left: Square Full-Height Tab Bar */}
        <div data-tauri-drag-region className="flex h-full items-stretch titlebar-drag-handle">
          <button
            onClick={() => setActiveSubTab('pipelines')}
            className={`h-full px-5 flex items-center space-x-2 border-r border-[#2b2b2b] text-xs font-bold transition-all relative ${
              activeSubTab === 'pipelines'
                ? 'bg-[#222225] text-cyan-400 border-b-2 border-b-cyan-400'
                : 'bg-transparent text-gray-400 hover:text-gray-200 hover:bg-white/5'
            }`}
          >
            <Sliders className="w-4 h-4" />
            <span>Filter Pipelines</span>
            <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-cyan-950/80 text-cyan-300 font-mono border border-cyan-800/60 ml-1">
              {filters.length}
            </span>
          </button>
          <button
            onClick={() => setActiveSubTab('operations')}
            className={`h-full px-5 flex items-center space-x-2 border-r border-[#2b2b2b] text-xs font-bold transition-all relative ${
              activeSubTab === 'operations'
                ? 'bg-[#222225] text-amber-400 border-b-2 border-b-amber-400'
                : 'bg-transparent text-gray-400 hover:text-gray-200 hover:bg-white/5'
            }`}
          >
            <Wrench className="w-4 h-4" />
            <span>Operations Library</span>
            <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-amber-950/80 text-amber-300 font-mono border border-amber-800/60 ml-1">
              {operationsCount}
            </span>
          </button>
        </div>

        {/* Right: Primary Action Button */}
        {activeSubTab === 'pipelines' ? (
          <button
            onClick={handleOpenCreateModal}
            className="flex items-center space-x-1.5 px-3.5 py-1.5 bg-white hover:bg-gray-200 text-black text-xs font-bold rounded-xl shadow-md active:scale-95 transition-all"
          >
            <Sparkles className="w-3.5 h-3.5 text-cyan-600" />
            <span>+ New Filter Pipeline</span>
          </button>
        ) : (
          <button
            onClick={() => openCreateOpRef.current?.()}
            className="flex items-center space-x-1.5 px-3.5 py-1.5 bg-white hover:bg-gray-200 text-black text-xs font-bold rounded-xl shadow-md active:scale-95 transition-all"
          >
            <Sparkles className="w-3.5 h-3.5 text-amber-500" />
            <span>+ New Operation</span>
          </button>
        )}
      </div>

      {/* Main Scrollable Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* Section Header below the 60px Section Picker */}
        <div className="pb-2 border-b border-[#2b2b2b]">
          {activeSubTab === 'pipelines' ? (
            <div>
              <h2 className="text-lg font-bold theme-title flex items-center space-x-2">
                <Sliders className="w-5 h-5 opacity-70 text-cyan-400" />
                <span>Filter Pipelines</span>
              </h2>
              <p className="text-xs theme-text-muted mt-1">
                Chain together reusable Operations to build multi-step text filtering & output automation workflows.
              </p>
            </div>
          ) : (
            <div>
              <h2 className="text-lg font-bold theme-title flex items-center space-x-2">
                <Wrench className="w-5 h-5 opacity-70 text-amber-400" />
                <span>Operations Library</span>
              </h2>
              <p className="text-xs theme-text-muted mt-1">
                Build and manage reusable regex replacements, built-in engine keys, and shell script operations.
              </p>
            </div>
          )}
        </div>

      {activeSubTab === 'operations' ? (
        <OperationsManager isEmbedded={true} openCreateRef={openCreateOpRef} />
      ) : (
        <>
          {/* Sticky Filter Sandbox */}
          <div className="sticky-filter-sandbox filter-sandbox-card sticky top-0 z-20 p-4 rounded-xl border space-y-3 shadow-xl backdrop-blur-xl">
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-semibold theme-text-muted uppercase tracking-wider flex items-center space-x-1.5 text-cyan-400">
                <Play className="w-3.5 h-3.5" />
                <span>Filter Sandbox</span>
              </h3>
              <span className="text-[10px] theme-text-muted">Click any filter below to test live</span>
            </div>
            <div className="grid grid-cols-2 gap-4 text-xs font-mono">
              <div>
                <label className="block theme-text-muted mb-1 font-sans">Input Text:</label>
                <textarea
                  value={testText}
                  onChange={(e) => setTestText(e.target.value)}
                  className="w-full h-24 theme-input border border-gray-700/80 rounded-lg p-2.5 focus:outline-none focus:border-gray-500 text-xs text-gray-200"
                />
              </div>
              <div>
                <label className="block theme-text-muted mb-1 font-sans font-semibold text-emerald-400">Output Preview:</label>
                <div className="w-full h-24 theme-input border border-gray-700/80 rounded-lg p-2.5 overflow-y-auto whitespace-pre-wrap text-emerald-300">
                  {testResult || 'Click any filter card below to test...'}
                </div>
              </div>
            </div>
          </div>

          {/* Filter Category Filter Pills */}
          <div className="flex items-center space-x-2 overflow-x-auto pb-1 scrollbar-none">
            {FILTER_CATEGORIES.map((cat) => (
              <button
                key={cat}
                onClick={() => setActiveCategory(cat)}
                className={`px-3 py-1 rounded-lg text-xs font-semibold whitespace-nowrap transition-all ${
                  activeCategory === cat
                    ? 'bg-cyan-600 text-white shadow'
                    : 'bg-[#212121] text-gray-400 hover:text-white hover:bg-gray-800'
                }`}
              >
                {cat}
              </button>
            ))}
          </div>

          {/* Active Filters Grid */}
          <div className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {filters
                .filter((f) => {
                  if (activeCategory === 'All') return true;
                  if (f.config) {
                    try {
                      const parsed = JSON.parse(f.config);
                      if (Array.isArray(parsed) && parsed.length > 0) {
                        return parsed.some((s: any) =>
                          (s.filter_type || '').toLowerCase().includes(activeCategory.toLowerCase().split(' ')[0])
                        );
                      }
                    } catch {}
                  }
                  return true;
                })
                .map((f) => {
                  let stepTypes: string[] = [f.filter_type];
                  if (f.config) {
                    try {
                      const parsed = JSON.parse(f.config);
                      if (Array.isArray(parsed) && parsed.length > 0) {
                        stepTypes = parsed.map((s: any) => s.filter_type);
                      }
                    } catch {}
                  }

                  return (
                    <div
                      key={f.id}
                      onClick={() => handleTestTransformation(f.filter_type, f.config)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setFilterContextMenu({ x: e.clientX, y: e.clientY, filter: f });
                      }}
                      className="group p-3.5 theme-card-idle bg-[#212121] rounded-xl border border-gray-700/80 hover:border-cyan-500 cursor-pointer transition-all flex items-center justify-between shadow-md"
                    >
                      <div className="flex items-center space-x-3 truncate pr-2">
                        <div className="p-2 rounded-lg theme-badge border shrink-0">
                          <Code2 className="w-4 h-4 text-cyan-400" />
                        </div>
                        <div className="truncate">
                          <div className="flex items-center space-x-2">
                            <h4 className="text-xs font-bold theme-text-main truncate">{f.name}</h4>
                            {stepTypes.length > 1 && (
                              <span className="text-[9px] font-bold text-cyan-400 bg-cyan-950/60 border border-cyan-800/60 px-1.5 py-0.2 rounded-full">
                                ⚡ {stepTypes.length} Steps
                              </span>
                            )}
                          </div>
                          <div className="flex items-center space-x-1.5 mt-1 overflow-x-auto scrollbar-none">
                            {stepTypes.map((st, i) => (
                              <React.Fragment key={i}>
                                {i > 0 && <span className="text-[10px] text-cyan-500/60 font-bold">➔</span>}
                                <span className="text-[10px] font-mono text-cyan-400/90 bg-cyan-950/40 px-1.5 py-0.5 rounded border border-cyan-800/40 whitespace-nowrap">
                                  {st}
                                </span>
                              </React.Fragment>
                            ))}
                          </div>
                        </div>
                      </div>

                  <div className="flex items-center space-x-2 shrink-0">
                    <div onClick={(e) => e.stopPropagation()}>
                      <HotkeyRecorder
                        value={f.shortcut}
                        onChange={async (newShortcut) => {
                          try {
                            await invoke('update_filter_shortcut', { id: f.id, shortcut: newShortcut });
                            onRefreshFilters();
                          } catch (err) {
                            console.error(err);
                          }
                        }}
                      />
                    </div>

                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleOpenEditModal(f);
                      }}
                      className="p-1.5 text-gray-400 hover:text-white rounded-md hover:bg-gray-800 transition-colors"
                      title="Edit Filter"
                    >
                      <Edit3 className="w-4 h-4" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteFilter(f.id);
                      }}
                      className="p-1.5 text-gray-400 hover:text-red-400 rounded-md hover:bg-gray-800 transition-colors"
                      title="Delete Filter"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              );
              })}
            </div>
          </div>

          {/* Floating Filter Context Menu */}
          {filterContextMenu && (
            <div
              className="fixed z-[99999] w-48 bg-[#1e2029]/95 backdrop-blur-xl border border-gray-700/80 rounded-xl shadow-2xl py-1.5 text-xs text-gray-200 animate-in fade-in duration-100 font-sans"
              style={{ top: filterContextMenu.y, left: filterContextMenu.x }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="px-3 py-1 text-[10px] uppercase font-bold text-gray-400 border-b border-gray-800 truncate">
                {filterContextMenu.filter.name}
              </div>

              <button
                onClick={() => {
                  handleOpenEditModal(filterContextMenu.filter);
                  setFilterContextMenu(null);
                }}
                className="w-full px-3 py-1.5 text-left flex items-center space-x-2 hover:bg-cyan-600 hover:text-white transition-colors"
              >
                <Edit3 className="w-3.5 h-3.5" />
                <span>Edit Filter</span>
              </button>

              <button
                onClick={() => {
                  handleDuplicateFilter(filterContextMenu.filter);
                  setFilterContextMenu(null);
                }}
                className="w-full px-3 py-1.5 text-left flex items-center space-x-2 hover:bg-cyan-600 hover:text-white transition-colors"
              >
                <Copy className="w-3.5 h-3.5" />
                <span>Duplicate Filter</span>
              </button>

              <button
                onClick={() => {
                  handleTestTransformation(filterContextMenu.filter.filter_type, filterContextMenu.filter.config);
                  setFilterContextMenu(null);
                }}
                className="w-full px-3 py-1.5 text-left flex items-center space-x-2 hover:bg-cyan-600 hover:text-white transition-colors"
              >
                <Play className="w-3.5 h-3.5" />
                <span>Test in Sandbox</span>
              </button>

              <button
                onClick={() => {
                  handleExportFilter(filterContextMenu.filter);
                  setFilterContextMenu(null);
                }}
                className="w-full px-3 py-1.5 text-left flex items-center space-x-2 hover:bg-cyan-600 hover:text-white transition-colors"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Export / Copy JSON</span>
              </button>

              <div className="my-1 border-t border-gray-800" />

              <button
                onClick={() => {
                  handleDeleteFilter(filterContextMenu.filter.id);
                  setFilterContextMenu(null);
                }}
                className="w-full px-3 py-1.5 text-left flex items-center space-x-2 text-red-400 hover:bg-red-600 hover:text-white transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Delete Filter</span>
              </button>
            </div>
          )}
        </>
      )}

      </div>

      {/* Editor Modal */}
      <FilterEditorModal
        filter={selectedFilterForEdit}
        isOpen={isEditorModalOpen}
        onClose={() => setIsEditorModalOpen(false)}
        onSaveSuccess={onRefreshFilters}
      />
    </div>
  );
};
