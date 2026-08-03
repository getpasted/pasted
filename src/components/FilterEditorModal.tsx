import React, { useState, useEffect, useRef } from 'react';
import { FilterRule, Operation } from '../types';
import { Sliders, Plus, Trash2, X, Play, ArrowDown, ArrowUp, GripVertical, Wrench, RotateCcw } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import { HotkeyRecorder } from './HotkeyRecorder';
import { OperationEditorModal } from './OperationEditorModal';
import { startWindowDrag } from '../utils/windowDrag';

export interface FilterStep {
  id: string;
  filter_type: string;
  config?: string | null;
  findPattern?: string;
  replacePattern?: string;
  matchMode?: 'regex' | 'literal' | 'wildcard';
  caseSensitive?: boolean;
  tagName?: string;
  shellCommand?: string;
}

interface FilterEditorModalProps {
  filter: FilterRule | null; // null if creating new
  isOpen: boolean;
  onClose: () => void;
  onSaveSuccess: () => void;
}

export const FILTER_TYPE_OPTIONS = [
  { value: 'regex', label: 'Find & Replace (Regex / Text)', category: 'Search' },
  { value: 'clean_url_tracking', label: 'Clean URL Tracking (Strip UTM)', category: 'Cleaners' },
  { value: 'strip_html', label: 'Plain Text / Strip HTML', category: 'Cleaners' },
  { value: 'strip_markdown', label: 'Strip Markdown Formatting', category: 'Cleaners' },
  { value: 'strip_emojis', label: 'Emoji Remover (Strip Emojis)', category: 'Cleaners' },
  { value: 'smileys_to_emoji', label: 'Convert Text Smileys to Emoji', category: 'Cleaners' },
  { value: 'smart_punctuation', label: 'Smart Punctuation (“ ” — …)', category: 'Format' },
  { value: 'straighten_punctuation', label: 'Straighten Punctuation (" \' -- ...)', category: 'Format' },
  { value: 'uppercase', label: 'UPPERCASE', category: 'Case' },
  { value: 'lowercase', label: 'lowercase', category: 'Case' },
  { value: 'titlecase', label: 'Title Case', category: 'Case' },
  { value: 'sentence_case', label: 'Sentence case', category: 'Case' },
  { value: 'camelcase', label: 'camelCase', category: 'Case' },
  { value: 'snakecase', label: 'snake_case', category: 'Case' },
  { value: 'kebabcase', label: 'kebab-case', category: 'Case' },
  { value: 'constant_case', label: 'CONSTANT_CASE', category: 'Case' },
  { value: 'alternating_case', label: 'aLtErNaTiNg cAsE', category: 'Case' },
  { value: 'extract_urls', label: 'Extract URLs', category: 'Extract' },
  { value: 'extract_emails', label: 'Extract Emails', category: 'Extract' },
  { value: 'extract_phones', label: 'Extract Phone Numbers', category: 'Extract' },
  { value: 'extract_ips', label: 'Extract IP Addresses', category: 'Extract' },
  { value: 'extract_numbers', label: 'Extract Numbers', category: 'Extract' },
  { value: 'sort_lines_asc', label: 'Sort Lines (A-Z)', category: 'Lines' },
  { value: 'sort_lines_desc', label: 'Sort Lines (Z-A)', category: 'Lines' },
  { value: 'sort_by_length', label: 'Sort Lines (By Length)', category: 'Lines' },
  { value: 'dedupe_lines', label: 'Deduplicate Lines', category: 'Lines' },
  { value: 'reverse_lines', label: 'Reverse Lines', category: 'Lines' },
  { value: 'strip_empty_lines', label: 'Strip Empty Lines', category: 'Lines' },
  { value: 'number_lines', label: 'Number Lines (1. 2. 3.)', category: 'Lines' },
  { value: 'quote_text', label: 'Quote Text (> )', category: 'Lines' },
  { value: 'wrap_tags', label: 'Wrap in HTML Tags', category: 'Structure' },
  { value: 'trim', label: 'Trim Whitespace', category: 'Cleaners' },
  { value: 'strip_newlines', label: 'Strip Newlines', category: 'Cleaners' },
  { value: 'json_format', label: 'Format JSON', category: 'Structure' },
  { value: 'json_minify', label: 'Minify JSON', category: 'Structure' },
  { value: 'html_encode', label: 'HTML Entity Encode', category: 'Encoding' },
  { value: 'html_decode', label: 'HTML Entity Decode', category: 'Encoding' },
  { value: 'hex_encode', label: 'Hex Encode', category: 'Encoding' },
  { value: 'hex_decode', label: 'Hex Decode', category: 'Encoding' },
  { value: 'url_encode', label: 'URL Encode', category: 'Encoding' },
  { value: 'url_decode', label: 'URL Decode', category: 'Encoding' },
  { value: 'shell_script', label: 'Shell Script Command (sh -c)', category: 'Advanced' },
];

const StepReorderCard: React.FC<{
  step: FilterStep;
  idx: number;
  totalSteps: number;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onInsertBelow: () => void;
  onRemove: () => void;
  onUpdate: (updates: Partial<FilterStep>) => void;
  operationsList: Operation[];
  setIsOpModalOpen: (open: boolean) => void;
  isDragging: boolean;
  reorderOffsetY: number;
  onReorderPointerDown: (event: React.PointerEvent) => void;
}> = ({
  step,
  idx,
  totalSteps,
  onMoveUp,
  onMoveDown,
  onInsertBelow,
  onRemove,
  onUpdate,
  operationsList,
  setIsOpModalOpen,
  isDragging,
  reorderOffsetY,
  onReorderPointerDown,
}) => {
  return (
      <div
        data-stable-reorder-id={step.id}
        style={reorderOffsetY !== 0 || isDragging ? {
          transform: `translateY(${reorderOffsetY}px)`,
          zIndex: isDragging ? 'var(--layer-drag)' : 1,
        } : undefined}
        className={`filter-step-card p-3.5 rounded-xl border space-y-3 relative group select-none transition-[background-color,border-color,box-shadow,opacity,transform] duration-100 ease-out ${
          isDragging ? 'is-dragging' : ''
        }`}
      >
        {/* Step Header */}
        <div className="flex items-center justify-between border-b border-gray-800/80 pb-2">
          {/* Left: Step Number Badge, Drag Handle, Arrow Buttons */}
          <div className="flex items-center space-x-1.5">
            <span className="w-5 h-5 rounded-full bg-cyan-950 text-cyan-300 text-[11px] font-bold flex items-center justify-center font-mono border border-cyan-700/60 mr-0.5">
              {idx + 1}
            </span>
            <button
              type="button"
              onPointerDown={onReorderPointerDown}
              className="step-drag-handle titlebar-no-drag p-1.5 text-gray-400 hover:text-white rounded hover:bg-gray-800/80 touch-none select-none shrink-0 border-0 outline-none"
              style={{ touchAction: 'none' }}
              title="Drag to reorder step"
            >
              <GripVertical className="w-4 h-4 pointer-events-none" />
            </button>
            <button
              type="button"
              disabled={idx === 0}
              onClick={onMoveUp}
              className="p-1 text-gray-400 hover:text-cyan-300 disabled:opacity-20 disabled:hover:text-gray-400 rounded hover:bg-gray-800 transition-colors"
              title="Move Step Up"
            >
              <ArrowUp className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              disabled={idx === totalSteps - 1}
              onClick={onMoveDown}
              className="p-1 text-gray-400 hover:text-cyan-300 disabled:opacity-20 disabled:hover:text-gray-400 rounded hover:bg-gray-800 transition-colors"
              title="Move Step Down"
            >
              <ArrowDown className="w-3.5 h-3.5" />
            </button>
          </div>

        {/* Right: Insert Below & Remove Step Actions */}
        <div className="flex items-center space-x-1.5">
          <button
            type="button"
            onClick={onInsertBelow}
            className="p-1 text-gray-400 hover:text-cyan-300 rounded hover:bg-gray-800 transition-colors"
            title="Insert Step Below"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>

          {totalSteps > 1 && (
            <button
              type="button"
              onClick={onRemove}
              className="p-1 text-gray-500 hover:text-red-400 rounded hover:bg-gray-800 transition-colors"
              title="Remove Step"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="block text-gray-400 theme-text-muted">Step Operation:</label>
            <button
              type="button"
              onClick={() => setIsOpModalOpen(true)}
              className="text-[10px] text-cyan-400 hover:text-cyan-300 flex items-center space-x-0.5 hover:underline"
              title="Create a new reusable operation"
            >
              <Wrench className="w-2.5 h-2.5" />
              <span>+ New Operation</span>
            </button>
          </div>
          <select
            value={step.filter_type}
            onChange={(e) => onUpdate({ filter_type: e.target.value })}
            className="w-full bg-[#242424] border border-gray-700/80 rounded-lg p-2 text-gray-200 focus:outline-none focus:border-cyan-500 theme-input font-sans"
          >
            {[
              { key: 'Search', label: 'Search & Replace' },
              { key: 'Cleaners', label: 'Cleaners & Sanitizers' },
              { key: 'Format', label: 'Smart Formatting' },
              { key: 'Case', label: 'Case Transformations' },
              { key: 'Extract', label: 'Data Extraction' },
              { key: 'Lines', label: 'Line Operations' },
              { key: 'Structure', label: 'Structure & Formatting' },
              { key: 'Encoding', label: 'Encodings & Decodings' },
              { key: 'Advanced', label: 'Advanced & Shell Scripts' },
            ].map((cat) => {
              const builtIns = FILTER_TYPE_OPTIONS.filter((opt) => opt.category === cat.key);
              return (
                <optgroup key={cat.key} label={cat.label}>
                  {builtIns.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </optgroup>
              );
            })}

            {operationsList.filter((o) => o.category === 'Custom Operations' || !['Cleaners & Sanitizers', 'Case Transformations', 'Smart Formatting', 'Data Extraction', 'Line Operations', 'Structure & Formatting', 'Encodings & Decodings', 'Advanced & Shell Scripts'].includes(o.category)).length > 0 && (
              <optgroup label="Custom Operations">
                {operationsList
                  .filter((o) => o.category === 'Custom Operations' || !['Cleaners & Sanitizers', 'Case Transformations', 'Smart Formatting', 'Data Extraction', 'Line Operations', 'Structure & Formatting', 'Encodings & Decodings', 'Advanced & Shell Scripts'].includes(o.category))
                  .map((op) => (
                    <option key={`custom-${op.id}`} value={op.op_type}>
                      {op.name}
                    </option>
                  ))}
              </optgroup>
            )}
          </select>
        </div>

        {/* Step Specific Config Inputs */}
        {step.filter_type === 'regex' && (
          <div className="space-y-2 col-span-2">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-gray-400 mb-1 theme-text-muted">Find:</label>
                <textarea
                  placeholder="Text pattern or Regex pattern"
                  value={step.findPattern || ''}
                  onChange={(e) => onUpdate({ findPattern: e.target.value })}
                  className="w-full h-16 bg-[#242424] border border-gray-700/80 rounded-lg p-2 font-mono text-xs text-gray-200 focus:outline-none focus:border-cyan-500 theme-input"
                />
              </div>
              <div>
                <label className="block text-gray-400 mb-1 theme-text-muted">Replace with:</label>
                <textarea
                  placeholder="Replacement string"
                  value={step.replacePattern || ''}
                  onChange={(e) => onUpdate({ replacePattern: e.target.value })}
                  className="w-full h-16 bg-[#242424] border border-gray-700/80 rounded-lg p-2 font-mono text-xs text-gray-200 focus:outline-none focus:border-cyan-500 theme-input"
                />
              </div>
            </div>
            <div className="flex items-center space-x-4 pt-1">
              <label className="flex items-center space-x-1.5 text-xs text-gray-400 cursor-pointer theme-text-muted">
                <input
                  type="checkbox"
                  checked={step.caseSensitive || false}
                  onChange={(e) => onUpdate({ caseSensitive: e.target.checked })}
                  className="rounded bg-[#242424] border-gray-700 text-cyan-600 focus:ring-0"
                />
                <span>Case Sensitive</span>
              </label>
            </div>
          </div>
        )}

        {step.filter_type === 'shell_script' && (
          <div className="col-span-2">
            <label className="block text-gray-400 mb-1 theme-text-muted">Shell Script Command (stdin -&gt; stdout):</label>
            <input
              type="text"
              placeholder='e.g. tr "a-z" "A-Z"'
              value={step.shellCommand || ''}
              onChange={(e) => onUpdate({ shellCommand: e.target.value })}
              className="w-full bg-[#242424] border border-gray-700/80 rounded-lg p-2 font-mono text-xs text-gray-200 focus:outline-none focus:border-cyan-500 theme-input"
            />
          </div>
        )}

        {step.filter_type === 'wrap_tags' && (
          <div>
            <label className="block text-gray-400 mb-1 theme-text-muted">HTML Tag Name:</label>
            <input
              type="text"
              placeholder="code, b, blockquote"
              value={step.tagName || ''}
              onChange={(e) => onUpdate({ tagName: e.target.value })}
              className="w-full bg-[#242424] border border-gray-700/80 rounded-lg p-2 font-mono text-xs text-gray-200 focus:outline-none focus:border-cyan-500 theme-input"
            />
          </div>
        )}
      </div>
    </div>
  );
};

export const FilterEditorModal: React.FC<FilterEditorModalProps> = ({
  filter,
  isOpen,
  onClose,
  onSaveSuccess,
}) => {
  const [filterName, setFilterName] = useState('');
  const [shortcut, setShortcut] = useState<string | null>(null);
  const [steps, setSteps] = useState<FilterStep[]>([]);
  const [testInput, setTestInput] = useState('Hello Pasted User! :) https://example.com?utm_source=test');
  const [testOutput, setTestOutput] = useState('');
  const [operationsList, setOperationsList] = useState<Operation[]>([]);
  const [isOpModalOpen, setIsOpModalOpen] = useState(false);
  const stepListRef = useRef<HTMLDivElement>(null);
  const {
    activeId: activeStepId,
    offsets: stepReorderOffsets,
    isSettling: isStepReorderSettling,
    startPointerReorder: startStepPointerReorder,
  } = useStableVerticalReorder({
    itemIds: steps.map((step) => step.id),
    containerRef: stepListRef,
    onCommit: (orderedIds) => {
      setSteps((current) => {
        const byId = new Map(current.map((step) => [step.id, step]));
        return orderedIds.map((id) => byId.get(id)).filter((step): step is FilterStep => Boolean(step));
      });
    },
  });

  const refreshOps = () => {
    invoke<Operation[]>('get_operations')
      .then((ops) => setOperationsList(ops))
      .catch(console.error);
  };

  useEffect(() => {
    if (!isOpen) return;

    // Fetch operations from SQLite database
    refreshOps();

    if (filter) {
      setFilterName(filter.name);
      setShortcut(filter.shortcut || null);
      if (filter.filter_type === 'pipeline' && filter.config) {
        try {
          const parsedSteps = JSON.parse(filter.config);
          setSteps(
            parsedSteps.map((s: any, index: number) => ({
              id: `step-${index}-${Date.now()}`,
              filter_type: s.filter_type,
              config: s.config,
              findPattern: s.findPattern || (s.config && typeof s.config === 'string' && s.config.includes('pattern') ? JSON.parse(s.config).pattern : ''),
              replacePattern: s.replacePattern || (s.config && typeof s.config === 'string' && s.config.includes('replacement') ? JSON.parse(s.config).replacement : ''),
              matchMode: s.matchMode || 'regex',
              caseSensitive: s.caseSensitive || false,
              tagName: s.tagName || (s.filter_type === 'wrap_tags' ? s.config || 'code' : 'code'),
              shellCommand: s.shellCommand || (s.filter_type === 'shell_script' ? s.config || 'tr "a-z" "A-Z"' : 'tr "a-z" "A-Z"'),
            }))
          );
        } catch {
          setSteps([createDefaultStep(filter.filter_type, filter.config)]);
        }
      } else {
        setSteps([createDefaultStep(filter.filter_type, filter.config)]);
      }
    } else {
      setFilterName('');
      setShortcut(null);
      setSteps([createDefaultStep('smart_punctuation', null)]);
    }
  }, [isOpen, filter]);

  const handleReset = () => {
    if (filter) {
      setFilterName(filter.name);
      setShortcut(filter.shortcut || null);
      if (filter.filter_type === 'pipeline' && filter.config) {
        try {
          const parsedSteps = JSON.parse(filter.config);
          setSteps(
            parsedSteps.map((s: any, index: number) => ({
              id: `step-${index}-${Date.now()}`,
              filter_type: s.filter_type,
              config: s.config,
              findPattern: s.findPattern || (s.config && typeof s.config === 'string' && s.config.includes('pattern') ? JSON.parse(s.config).pattern : ''),
              replacePattern: s.replacePattern || (s.config && typeof s.config === 'string' && s.config.includes('replacement') ? JSON.parse(s.config).replacement : ''),
              matchMode: s.matchMode || 'regex',
              caseSensitive: s.caseSensitive || false,
              tagName: s.tagName || (s.filter_type === 'wrap_tags' ? s.config || 'code' : 'code'),
              shellCommand: s.shellCommand || (s.filter_type === 'shell_script' ? s.config || 'tr "a-z" "A-Z"' : 'tr "a-z" "A-Z"'),
            }))
          );
        } catch {
          setSteps([createDefaultStep(filter.filter_type, filter.config)]);
        }
      } else {
        setSteps([createDefaultStep(filter.filter_type, filter.config)]);
      }
    } else {
      setFilterName('');
      setShortcut(null);
      setSteps([createDefaultStep('smart_punctuation', null)]);
      setTestInput('Hello Pasted User! :) https://example.com?utm_source=test');
    }
  };

  // Run live test execution when steps or testInput change
  useEffect(() => {
    if (!isOpen) return;
    runLiveTest();
  }, [steps, testInput, isOpen]);

  const createDefaultStep = (filterType: string, config: string | null): FilterStep => {
    return {
      id: `step-${Date.now()}-${Math.random()}`,
      filter_type: filterType,
      config,
      findPattern: '',
      replacePattern: '',
      matchMode: 'regex',
      caseSensitive: false,
      tagName: 'code',
      shellCommand: 'tr "a-z" "A-Z"',
    };
  };

  const handleAddStep = () => {
    setSteps((prev) => [...prev, createDefaultStep('smart_punctuation', null)]);
  };

  const handleMoveStepUp = (index: number) => {
    if (index <= 0) return;
    setSteps((prev) => {
      const copy = [...prev];
      const temp = copy[index - 1];
      copy[index - 1] = copy[index];
      copy[index] = temp;
      return copy;
    });
  };

  const handleMoveStepDown = (index: number) => {
    if (index >= steps.length - 1) return;
    setSteps((prev) => {
      const copy = [...prev];
      const temp = copy[index + 1];
      copy[index + 1] = copy[index];
      copy[index] = temp;
      return copy;
    });
  };

  const handleInsertStepAt = (index: number) => {
    setSteps((prev) => {
      const copy = [...prev];
      copy.splice(index, 0, createDefaultStep('trim', null));
      return copy;
    });
  };

  const handleRemoveStep = (id: string) => {
    if (steps.length === 1) return; // Keep at least one step
    setSteps((prev) => prev.filter((s) => s.id !== id));
  };

  const handleUpdateStep = (id: string, updates: Partial<FilterStep>) => {
    setSteps((prev) =>
      prev.map((s) => (s.id === id ? { ...s, ...updates } : s))
    );
  };

  const runLiveTest = async () => {
    if (!testInput) {
      setTestOutput('');
      return;
    }
    try {
      let current = testInput;
      for (const step of steps) {
        let stepConfig: string | null = null;
        if (step.filter_type === 'regex') {
          stepConfig = JSON.stringify({
            pattern: step.findPattern || '',
            replacement: step.replacePattern || '',
            matchMode: step.matchMode || 'regex',
            caseSensitive: step.caseSensitive || false,
          });
        } else if (step.filter_type === 'wrap_tags') {
          stepConfig = step.tagName || 'code';
        } else if (step.filter_type === 'shell_script') {
          stepConfig = step.shellCommand || 'cat';
        } else if (step.config) {
          stepConfig = step.config;
        }

        current = await invoke<string>('transform_text', {
          input: current,
          filterType: step.filter_type,
          config: stepConfig,
        });
      }
      setTestOutput(current);
    } catch (e) {
      setTestOutput(`Error: ${e}`);
    }
  };

  const handleSaveFilter = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!filterName.trim()) return;

    try {
      const compiledSteps = steps.map((s) => {
        let stepConfig: string | null = null;
        if (s.filter_type === 'regex') {
          stepConfig = JSON.stringify({
            pattern: s.findPattern || '',
            replacement: s.replacePattern || '',
            matchMode: s.matchMode || 'regex',
            caseSensitive: s.caseSensitive || false,
          });
        } else if (s.filter_type === 'wrap_tags') {
          stepConfig = s.tagName || 'code';
        } else if (s.filter_type === 'shell_script') {
          stepConfig = s.shellCommand || 'cat';
        } else {
          stepConfig = s.config || null;
        }

        return {
          filter_type: s.filter_type,
          config: stepConfig,
          findPattern: s.findPattern,
          replacePattern: s.replacePattern,
          matchMode: s.matchMode,
          caseSensitive: s.caseSensitive,
          tagName: s.tagName,
          shellCommand: s.shellCommand,
        };
      });

      const isSingleSimple = compiledSteps.length === 1 && compiledSteps[0].filter_type !== 'regex';
      const finalFilterType = isSingleSimple ? compiledSteps[0].filter_type : 'pipeline';
      const finalConfig = isSingleSimple ? compiledSteps[0].config : JSON.stringify(compiledSteps);

      if (filter) {
        // Edit existing filter
        await invoke('delete_filter', { id: filter.id });
        await invoke('create_filter', {
          name: filterName.trim(),
          filterType: finalFilterType,
          config: finalConfig,
          shortcut,
        });
      } else {
        // Create new filter
        await invoke('create_filter', {
          name: filterName.trim(),
          filterType: finalFilterType,
          config: finalConfig,
          shortcut,
        });
      }

      onSaveSuccess();
      onClose();
    } catch (e) {
      console.error(e);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="app-dialog-overlay fixed inset-0 flex items-center justify-center p-6 animate-in fade-in duration-150">
      <div className="filter-editor-card w-full max-w-4xl max-h-[90vh] border rounded-2xl flex flex-col overflow-hidden">
        {/* Modal Top Header Bar */}
        <div onMouseDown={startWindowDrag} className="filter-editor-header px-6 py-4 border-b flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-2 rounded-xl bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
              <Sliders className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-bold text-gray-100 theme-title">
                {filter ? 'Edit Filter' : 'New Filter'}
              </h3>
              <p className="text-xs text-gray-400 theme-text-muted">
                Chain multiple text operations into a single automated filter pipeline.
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Modal Body */}
        <div className="filter-editor-body flex-1 overflow-y-auto p-6 space-y-6 relative">
          {/* Filter Metadata */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
            <div className="md:col-span-2">
              <label className="block text-xs font-semibold text-gray-300 uppercase tracking-wider mb-2 theme-text-muted">
                Filter Name:
              </label>
              <input
                type="text"
                placeholder="e.g. Sanitize HTML & Convert Smileys"
                value={filterName}
                onChange={(e) => setFilterName(e.target.value)}
                className="w-full bg-[#181818] border border-gray-700/80 rounded-xl px-4 py-2.5 text-sm text-gray-100 focus:outline-none focus:border-cyan-500 font-medium theme-input"
                autoFocus
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-gray-300 uppercase tracking-wider mb-2 theme-text-muted">
                Global Hotkey Shortcut:
              </label>
              <div className="bg-[#181818] border border-gray-700/80 rounded-xl p-2 flex items-center justify-between theme-input">
                <span className="text-xs text-gray-400 theme-text-muted">Shortcut:</span>
                <HotkeyRecorder
                  value={shortcut}
                  placeholder="+ Set Hotkey"
                  onChange={(newShortcut) => setShortcut(newShortcut)}
                />
              </div>
            </div>
          </div>

          {/* Sticky Interactive Split-Pane Sandbox Tester */}
          <div className="filter-sandbox-card sticky-filter-sandbox sticky top-0 p-4 rounded-2xl border space-y-3 shadow-xl backdrop-blur-xl">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold text-cyan-400 uppercase tracking-wider flex items-center space-x-1.5">
                <Play className="w-3.5 h-3.5" />
                <span>Sticky Live Sandbox Tester</span>
              </span>
              <span className="text-[10px] text-gray-400 theme-text-muted">Live preview updates automatically on step edit</span>
            </div>

            <div className="grid grid-cols-2 gap-4 text-xs font-mono">
              <div>
                <label className="block text-gray-400 mb-1 font-sans theme-text-muted">Input Text:</label>
                <textarea
                  value={testInput}
                  onChange={(e) => setTestInput(e.target.value)}
                  className="w-full h-24 bg-[#242424] border border-gray-700/80 rounded-xl p-2.5 focus:outline-none focus:border-cyan-500 text-gray-200 theme-input"
                />
              </div>
              <div>
                <label className="filter-sandbox-output-label block mb-1 font-sans font-semibold">Live Output Preview:</label>
                <div className="filter-sandbox-output w-full h-24 border rounded-xl p-2.5 overflow-y-auto whitespace-pre-wrap theme-input font-mono">
                  {testOutput || 'Filtered output result will appear here...'}
                </div>
              </div>
            </div>
          </div>

          {/* Sequential Step Builder */}
          <div className="space-y-3 pt-2">
            <div className="flex items-center justify-between">
              <label className="text-xs font-semibold text-gray-300 uppercase tracking-wider theme-text-muted">
                Pipeline Execution Steps ({steps.length})
              </label>
            </div>

            {/* Dark Wrapper Container */}
            <div className="filter-step-list p-3 rounded-2xl border space-y-2.5 shadow-inner">
              <div
                ref={stepListRef}
                className={`space-y-2.5 ${isStepReorderSettling ? 'is-settling-stable-reorder' : ''}`}
              >
                {steps.map((step, idx) => (
                  <StepReorderCard
                    key={step.id}
                    step={step}
                    idx={idx}
                    totalSteps={steps.length}
                    onMoveUp={() => handleMoveStepUp(idx)}
                    onMoveDown={() => handleMoveStepDown(idx)}
                    onInsertBelow={() => handleInsertStepAt(idx + 1)}
                    onRemove={() => handleRemoveStep(step.id)}
                    onUpdate={(updates) => handleUpdateStep(step.id, updates)}
                    operationsList={operationsList}
                    setIsOpModalOpen={setIsOpModalOpen}
                    isDragging={activeStepId === step.id}
                    reorderOffsetY={stepReorderOffsets[step.id] ?? 0}
                    onReorderPointerDown={(event) => startStepPointerReorder(step.id, event)}
                  />
                ))}
              </div>

              {/* Bottom Add Step Button inside dark wrapper */}
              <div className="pt-1 flex justify-center">
                <button
                  type="button"
                  onClick={handleAddStep}
                  className="flex items-center space-x-1.5 px-4 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-semibold shadow-lg active:scale-95 transition-[background-color,transform]"
                >
                  <Plus className="w-4 h-4" />
                  <span>Add Step</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Modal Bottom Footer Actions */}
        <div className="filter-editor-footer px-6 py-4 border-t flex items-center justify-between">
          <button
            type="button"
            onClick={handleReset}
            className="flex items-center space-x-1.5 px-3.5 py-2 bg-gray-800/80 hover:bg-gray-700 text-gray-300 hover:text-white rounded-xl text-xs font-medium transition-colors border border-gray-700/60"
            title="Reset form to original filter state"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset</span>
          </button>

          <div className="flex items-center space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="filter-modal-cancel-btn px-4 py-2 rounded-xl text-xs font-medium transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleSaveFilter}
              className="filter-modal-ok-btn px-5 py-2 rounded-xl text-xs font-bold shadow-lg transition-[background-color,border-color,color,transform] active:scale-95"
            >
              Save Filter Pipeline
            </button>
          </div>
        </div>
      </div>

      {/* Embedded Operation Editor Modal */}
      <OperationEditorModal
        operation={null}
        isOpen={isOpModalOpen}
        onClose={() => setIsOpModalOpen(false)}
        onSaveSuccess={refreshOps}
      />
    </div>
  );
};
