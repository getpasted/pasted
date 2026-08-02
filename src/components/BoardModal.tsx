import React, { useState, useEffect } from 'react';
import { Plus, Minus } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { Board } from '../types';
import { formatEmojiIcon } from '../utils/emoji';

interface BoardModalProps {
  isOpen: boolean;
  editingBoard?: Board | null;
  onClose: () => void;
  onRefreshBoards: () => void;
}

interface SmartConditionRow {
  id: string;
  target: 'source_app' | 'content_type' | 'contains';
  operator: 'is' | 'contains';
  value: string;
}

const COLOR_PALETTE = [
  { hex: '#ef4444', label: 'Red' },
  { hex: '#f97316', label: 'Orange' },
  { hex: '#eab308', label: 'Yellow' },
  { hex: '#10b981', label: 'Green' },
  { hex: '#ec4899', label: 'Pink' },
  { hex: '#8b5cf6', label: 'Purple' },
  { hex: '#06b6d4', label: 'Cyan' },
  { hex: '#3b82f6', label: 'Blue' },
  { hex: '#6b7280', label: 'Gray' },
  { hex: '#d97706', label: 'Amber' },
];

export const BoardModal: React.FC<BoardModalProps> = ({
  isOpen,
  editingBoard,
  onClose,
  onRefreshBoards,
}) => {
  const [modalTab, setModalTab] = useState<'pasteboard' | 'smart' | 'filter'>('pasteboard');
  const [name, setName] = useState('');
  const [selectedColor, setSelectedColor] = useState('#3b82f6');
  const [icon, setIcon] = useState('📂');

  // Form Validation State
  const [errors, setErrors] = useState<{ name?: boolean; color?: boolean; icon?: boolean }>({});

  // Installed OS Apps state
  const [installedApps, setInstalledApps] = useState<string[]>([]);

  // Multi-condition Smart Rules state
  const [conditions, setConditions] = useState<SmartConditionRow[]>([
    { id: '1', target: 'source_app', operator: 'is', value: '1Password' },
  ]);
  const [matchCondition, setMatchCondition] = useState<'any' | 'all'>('any');

  const modalRef = React.useRef<HTMLDivElement>(null);

  // Focus Trap & Escape Key Listener
  useEffect(() => {
    if (!isOpen) return;

    const previousFocus = document.activeElement as HTMLElement | null;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        return;
      }

      if (e.key === 'Tab' && modalRef.current) {
        const focusables = modalRef.current.querySelectorAll<HTMLElement>(
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
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (previousFocus && typeof previousFocus.focus === 'function') {
        previousFocus.focus();
      }
    };
  }, [isOpen, onClose]);

  useEffect(() => {
    if (isOpen) {
      setErrors({});

      invoke<string[]>('get_installed_applications')
        .then((apps) => {
          setInstalledApps(apps);
          if (apps.length > 0 && conditions[0].value === '1Password' && !apps.includes('1Password')) {
            setConditions((prev) =>
              prev.map((c, idx) => (idx === 0 ? { ...c, value: apps[0] } : c))
            );
          }
        })
        .catch(console.error);

      if (editingBoard) {
        setName(editingBoard.name);
        setSelectedColor(editingBoard.color || '#3b82f6');
        setIcon(formatEmojiIcon(editingBoard.icon));
        if (editingBoard.smart_rule) {
          setModalTab('smart');
          try {
            const parsed = JSON.parse(editingBoard.smart_rule);
            if (parsed.conditions && parsed.conditions.length > 0) {
              setConditions(
                parsed.conditions.map((c: any, i: number) => ({
                  id: String(i + 1),
                  target: c.type || 'source_app',
                  operator: c.operator || 'is',
                  value: c.value || '',
                }))
              );
            }
            setMatchCondition(parsed.match || 'any');
          } catch (e) {
            console.error(e);
          }
        } else {
          setModalTab('pasteboard');
        }
      } else {
        setName('');
        setSelectedColor('#3b82f6');
        setIcon('📂');
        setModalTab('pasteboard');
        setConditions([{ id: '1', target: 'source_app', operator: 'is', value: '1Password' }]);
      }
    }
  }, [isOpen, editingBoard]);

  if (!isOpen) return null;

  const handleAddCondition = () => {
    const defaultVal = installedApps[0] || 'Safari';
    setConditions((prev) => [
      ...prev,
      {
        id: String(Date.now() + Math.random()),
        target: 'source_app',
        operator: 'is',
        value: defaultVal,
      },
    ]);
  };

  const handleRemoveCondition = (id: string) => {
    if (conditions.length <= 1) return;
    setConditions((prev) => prev.filter((c) => c.id !== id));
  };

  const handleUpdateCondition = (id: string, updates: Partial<SmartConditionRow>) => {
    setConditions((prev) =>
      prev.map((c) => (c.id === id ? { ...c, ...updates } : c))
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validation Check
    const newErrors: { name?: boolean; color?: boolean; icon?: boolean } = {};
    if (!name.trim()) newErrors.name = true;
    if (!selectedColor) newErrors.color = true;
    if (!icon || !icon.trim()) newErrors.icon = true;

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }

    setErrors({});

    let smartRuleJson: string | null = null;
    if (modalTab === 'smart') {
      smartRuleJson = JSON.stringify({
        conditions: conditions.map((c) => ({
          type: c.target,
          operator: c.operator,
          value: c.value.trim(),
        })),
        match: matchCondition,
      });
    }

    try {
      if (editingBoard) {
        await invoke('update_board', {
          id: editingBoard.id,
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
      } else {
        await invoke('create_board', {
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
      }
      setName('');
      onRefreshBoards();
      onClose();
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div ref={modalRef} className="fixed inset-0 bg-black/75 backdrop-blur-md z-50 flex items-center justify-center p-4 select-none">
      <div className="board-modal-card bg-[#212121] w-full max-w-xl rounded-2xl p-6 space-y-5 border border-gray-700/80 shadow-2xl text-gray-100 font-sans">
        {/* Top Segmented Tab Picker */}
        <div className="flex justify-center">
          <div className="flex theme-surface bg-[#181818] p-1 rounded-xl border border-gray-700/70 space-x-1">
            <button
              type="button"
              onClick={() => setModalTab('pasteboard')}
              className={`px-4 py-1.5 rounded-lg text-xs font-semibold transition-all border ${
                modalTab === 'pasteboard'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              Pasteboard
            </button>
            <button
              type="button"
              onClick={() => setModalTab('smart')}
              className={`px-4 py-1.5 rounded-lg text-xs font-semibold transition-all border ${
                modalTab === 'smart'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              Smart Pasteboard
            </button>
            <button
              type="button"
              onClick={() => setModalTab('filter')}
              className={`px-4 py-1.5 rounded-lg text-xs font-semibold transition-all border ${
                modalTab === 'filter'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              Filterboard
            </button>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4 text-xs">
          {/* Name Field */}
          <div className="flex items-center space-x-3">
            <label className={`w-14 text-right font-semibold flex-shrink-0 ${errors.name ? 'text-red-500 font-bold dark:text-red-400' : 'theme-text-muted'}`}>Name:</label>
            <input
              type="text"
              placeholder="e.g. Code Snippets, Safari Clips"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                if (errors.name) setErrors((prev) => ({ ...prev, name: false }));
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleSubmit(e);
                }
              }}
              className={`flex-1 theme-input bg-[#181920] rounded-xl px-3 py-2 text-xs focus:outline-none font-medium transition-all ${
                errors.name
                  ? 'border border-red-500 ring-2 ring-red-500/30 dark:ring-red-500/50 bg-red-500/10'
                  : 'border border-blue-500/80 focus:ring-2 focus:ring-blue-500/60'
              }`}
              autoFocus
            />
          </div>

          {/* Color Palette Picker Row */}
          <div className="flex items-center space-x-3">
            <label className={`w-14 text-right font-semibold flex-shrink-0 ${errors.color ? 'text-red-500 font-bold dark:text-red-400' : 'theme-text-muted'}`}>Color:</label>
            <div className={`flex items-center space-x-2 p-1 rounded-xl transition-all ${errors.color ? 'border border-red-500 ring-2 ring-red-500/30 dark:ring-red-500/50 bg-red-500/10' : ''}`}>
              {COLOR_PALETTE.map((c) => (
                <button
                  key={c.hex}
                  type="button"
                  onClick={() => {
                    setSelectedColor(c.hex);
                    if (errors.color) setErrors((prev) => ({ ...prev, color: false }));
                  }}
                  style={{ backgroundColor: c.hex }}
                  className={`w-5 h-5 rounded-full transition-transform ${
                    selectedColor === c.hex
                      ? 'ring-2 ring-blue-500 ring-offset-2 scale-110'
                      : 'opacity-80 hover:opacity-100'
                  }`}
                  title={c.label}
                />
              ))}
            </div>
          </div>

          {/* Single Emoji Icon Selector */}
          <div className="flex items-center space-x-3">
            <label className={`w-14 text-right font-semibold flex-shrink-0 ${errors.icon ? 'text-red-500 font-bold dark:text-red-400' : 'theme-text-muted'}`}>Icon:</label>
            <div className="flex-1 flex items-center space-x-2.5">
              <input
                type="text"
                value={formatEmojiIcon(icon)}
                onChange={(e) => {
                  const val = e.target.value;
                  if (!val) return;
                  try {
                    const IntlAny = Intl as any;
                    if (typeof IntlAny.Segmenter === 'function') {
                      const segmenter = new IntlAny.Segmenter(undefined, { granularity: 'grapheme' });
                      const segments = Array.from(segmenter.segment(val)) as Array<{ segment: string }>;
                      if (segments.length > 0) {
                        const lastGrapheme = segments[segments.length - 1].segment;
                        if (lastGrapheme && lastGrapheme.trim()) {
                          setIcon(lastGrapheme);
                          if (errors.icon) setErrors((prev) => ({ ...prev, icon: false }));
                          return;
                        }
                      }
                    }
                  } catch (err) {
                    console.error(err);
                  }
                  const chars = Array.from(val);
                  const newEmoji = chars[chars.length - 1];
                  if (newEmoji) {
                    setIcon(newEmoji);
                    if (errors.icon) setErrors((prev) => ({ ...prev, icon: false }));
                  }
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    return; // Allow form submit
                  }
                  if (e.key !== 'Tab' && !e.metaKey && !e.ctrlKey) {
                    e.preventDefault();
                  }
                }}
                onClick={(e) => {
                  (e.target as HTMLInputElement).select();
                  invoke('open_emoji_picker').catch(() => {});
                }}
                onFocus={(e) => {
                  (e.target as HTMLInputElement).select();
                  invoke('open_emoji_picker').catch(() => {});
                }}
                onBlur={() => {
                  if (!icon) setIcon('📂');
                }}
                placeholder="📂"
                maxLength={64}
                className={`w-16 theme-input emoji-input-picker bg-[#181920] rounded-xl py-1.5 text-center font-mono text-lg focus:outline-none shadow-inner cursor-pointer select-none transition-all ${
                  errors.icon
                    ? 'border border-red-500 ring-2 ring-red-500/30 dark:ring-red-500/50 bg-red-500/10'
                    : 'border border-gray-700 focus:ring-2 focus:ring-blue-500/60'
                }`}
                title="Click to open macOS Emoji Picker (Manual typing disabled)"
              />
              <span className="text-[11px] theme-text-muted">
                Click input to open Emoji picker <kbd className="font-mono text-[10px] px-1.5 py-0.5 rounded theme-badge border">⌘Control+Space</kbd>
              </span>
            </div>
          </div>

          {/* Smart Pasteboard Multi-Condition Builder */}
          {modalTab === 'smart' && (
            <div className="p-4 theme-surface bg-[#1b1c24] rounded-2xl border border-gray-700/80 space-y-3">
              {conditions.map((c) => (
                <div key={c.id} className="flex items-center space-x-2">
                  {/* Condition Target Dropdown */}
                  <select
                    value={c.target}
                    onChange={(e) => {
                      const newTarget = e.target.value as any;
                      const newDefaultVal =
                        newTarget === 'source_app'
                          ? installedApps[0] || 'Safari'
                          : newTarget === 'content_type'
                          ? 'code'
                          : '';
                      handleUpdateCondition(c.id, { target: newTarget, value: newDefaultVal });
                    }}
                    className="theme-input bg-[#242630] border border-gray-700 rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                  >
                    <option value="source_app">Source App</option>
                    <option value="content_type">Content Type</option>
                    <option value="contains">Text Content</option>
                  </select>

                  {/* Operator Dropdown */}
                  <select
                    value={c.operator}
                    onChange={(e) => handleUpdateCondition(c.id, { operator: e.target.value as any })}
                    className="theme-input bg-[#242630] border border-gray-700 rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                  >
                    <option value="is">is</option>
                    <option value="contains">contains</option>
                  </select>

                  {/* Dynamic Value Dropdown / Input */}
                  {c.target === 'source_app' ? (
                    <select
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value })}
                      className="flex-1 theme-input bg-[#242630] border border-gray-700 rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none truncate"
                    >
                      {installedApps.map((appName) => (
                        <option key={appName} value={appName}>
                          {appName}
                        </option>
                      ))}
                    </select>
                  ) : c.target === 'content_type' ? (
                    <select
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value })}
                      className="flex-1 theme-input bg-[#242630] border border-gray-700 rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                    >
                      <option value="code">Code Snippets</option>
                      <option value="link">Web Links</option>
                      <option value="color">Hex Colors</option>
                      <option value="image">Images</option>
                    </select>
                  ) : (
                    <input
                      type="text"
                      placeholder="e.g. http, function"
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value })}
                      className="flex-1 theme-input bg-[#242630] border border-gray-700 rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                    />
                  )}

                  {/* Active + / - Condition Buttons */}
                  <div className="flex items-center space-x-1">
                    <button
                      type="button"
                      onClick={() => handleRemoveCondition(c.id)}
                      disabled={conditions.length <= 1}
                      className={`p-1.5 rounded theme-input bg-[#242630] border border-gray-700 transition-all ${
                        conditions.length <= 1
                          ? 'opacity-40 cursor-not-allowed text-gray-500'
                          : 'hover:bg-gray-400/20 hover:scale-105 active:scale-95'
                      }`}
                      title="Remove Condition"
                    >
                      <Minus className="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      onClick={handleAddCondition}
                      className="p-1.5 rounded theme-input bg-[#242630] border border-gray-700 hover:bg-blue-600 hover:text-white hover:border-blue-500 transition-all hover:scale-105 active:scale-95"
                      title="Add Condition"
                    >
                      <Plus className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}

              <div className="flex items-center space-x-2 pt-1 theme-text-muted">
                <span>Contain clippings that match</span>
                <select
                  value={matchCondition}
                  onChange={(e) => setMatchCondition(e.target.value as any)}
                  className="theme-input bg-[#242630] border border-gray-700 rounded-lg px-3 py-1 text-xs font-semibold focus:outline-none"
                >
                  <option value="any">any</option>
                  <option value="all">all</option>
                </select>
                <span>conditions</span>
              </div>
            </div>
          )}

          {/* Action Buttons */}
          <div className="pt-3 flex justify-end space-x-3 border-t border-gray-700/80">
            <button
              type="button"
              onClick={onClose}
              className="board-modal-cancel-btn px-5 py-2 rounded-xl bg-gray-800 hover:bg-gray-700 text-gray-300 font-semibold text-xs transition-all focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2 focus:ring-offset-[#212121]"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="board-modal-ok-btn px-5 py-2 rounded-xl bg-white hover:bg-gray-200 text-black font-semibold text-xs shadow-md transition-all active:scale-95 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-[#212121]"
            >
              {editingBoard ? 'Save Changes' : 'OK'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
