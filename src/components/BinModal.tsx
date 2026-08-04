import React, { useState, useEffect } from 'react';
import { Folder, Plus, Minus } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { Bin, SavedTransform } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';

interface BinModalProps {
  isOpen: boolean;
  editingBin?: Bin | null;
  onClose: () => void;
  onRefreshBins: () => void;
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

function initialBinForm(editingBin?: Bin | null) {
  if (editingBin?.smart_rule) {
    try {
      const parsed = JSON.parse(editingBin.smart_rule);
      return {
        modalTab: 'smart',
        conditions: parsed.conditions?.length > 0
          ? parsed.conditions.map((condition: any, index: number) => ({
            id: String(index + 1),
            target: condition.type || 'source_app',
            operator: condition.operator || 'is',
            value: condition.value || '',
          }))
          : [{ id: '1', target: 'source_app', operator: 'is', value: '' }],
        matchCondition: parsed.match || 'any',
      };
    } catch {
      // Fall through to the safe manual defaults used by the editor.
    }
  }
  return {
    modalTab: 'bin',
    conditions: [{ id: '1', target: 'source_app', operator: 'is', value: editingBin ? '' : '1Password' }],
    matchCondition: 'any',
  };
}

export const BinModal: React.FC<BinModalProps> = ({
  isOpen,
  editingBin,
  onClose,
  onRefreshBins,
}) => {
  const [modalTab, setModalTab] = useState<'bin' | 'smart'>(() => {
    if (editingBin?.smart_rule) return 'smart';
    return 'bin';
  });
  const [name, setName] = useState(() => editingBin?.name || '');
  const [selectedColor, setSelectedColor] = useState(() => editingBin?.color || '#3b82f6');
  const [icon, setIcon] = useState(() => (editingBin ? formatEmojiIcon(editingBin.icon) : '📂'));

  // Form Validation State
  const [errors, setErrors] = useState<{ name?: boolean; color?: boolean; icon?: boolean }>({});

  // Installed OS Apps state
  const [installedApps, setInstalledApps] = useState<string[]>([]);
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [transformRef, setTransformRef] = useState('');

  // Multi-condition Smart Rules state
  const [conditions, setConditions] = useState<SmartConditionRow[]>(() => {
    if (editingBin?.smart_rule) {
      try {
        const parsed = JSON.parse(editingBin.smart_rule);
        if (parsed.conditions && parsed.conditions.length > 0) {
          return parsed.conditions.map((c: any, i: number) => ({
            id: String(i + 1),
            target: c.type || 'source_app',
            operator: c.operator || 'is',
            value: c.value || '',
          }));
        }
      } catch (e) {
        console.error(e);
      }
    }
    return [{ id: '1', target: 'source_app', operator: 'is', value: '1Password' }];
  });
  const [matchCondition, setMatchCondition] = useState<'any' | 'all'>(() => {
    if (editingBin?.smart_rule) {
      try {
        const parsed = JSON.parse(editingBin.smart_rule);
        return parsed.match || 'any';
      } catch (e) {
        console.error(e);
      }
    }
    return 'any';
  });

  const initialTransformRef = React.useRef('');

  useEffect(() => {
    if (isOpen) {
      setErrors({});

      if (editingBin) {
        setName(editingBin.name);
        setSelectedColor(editingBin.color || '#3b82f6');
        setIcon(formatEmojiIcon(editingBin.icon));
        if (editingBin.smart_rule) {
          setModalTab('smart');
          try {
            const parsed = JSON.parse(editingBin.smart_rule);
            if (parsed.conditions && parsed.conditions.length > 0) {
              setConditions(
                parsed.conditions.map((c: any, i: number) => ({
                  id: String(i + 1),
                  target: c.type || 'source_app',
                  operator: c.operator || 'is',
                  value: c.value || '',
                }))
              );
            } else {
              setConditions([{ id: '1', target: 'source_app', operator: 'is', value: '' }]);
            }
            setMatchCondition(parsed.match || 'any');
          } catch (e) {
            console.error(e);
            setConditions([{ id: '1', target: 'source_app', operator: 'is', value: '' }]);
          }
        } else {
          setModalTab('bin');
          setConditions([{ id: '1', target: 'source_app', operator: 'is', value: '' }]);
        }
      } else {
        setName('');
        setSelectedColor('#3b82f6');
        setIcon('📂');
        setModalTab('bin');
        setConditions([{ id: '1', target: 'source_app', operator: 'is', value: '1Password' }]);
      }

      invoke<string[]>('get_installed_applications')
        .then((apps) => {
          setInstalledApps(apps);
        })
        .catch(console.error);
      invoke<SavedTransform[]>('get_saved_transforms')
        .then(setTransforms)
        .catch(console.error);
      if (editingBin) {
        invoke<string | null>('get_bin_transform_ref', { binId: editingBin.id })
          .then((value) => {
            initialTransformRef.current = value || '';
            setTransformRef(value || '');
          })
          .catch(console.error);
      } else {
        initialTransformRef.current = '';
        setTransformRef('');
      }
    }
  }, [isOpen, editingBin]);

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
      if (editingBin) {
        await invoke('update_bin', {
          id: editingBin.id,
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
        await invoke('set_bin_transform_ref', { binId: editingBin.id, transformRef: transformRef || null });
      } else {
        const created = await invoke<Bin>('create_bin', {
          name: name.trim(),
          icon: icon || '📂',
          color: selectedColor,
          smartRule: smartRuleJson,
        });
        await invoke('set_bin_transform_ref', { binId: created.id, transformRef: transformRef || null });
      }
      setName('');
      onRefreshBins();
      onClose();
    } catch (err) {
      console.error(err);
    }
  };

  const initial = initialBinForm(editingBin);
  const isDirty = JSON.stringify({
    modalTab,
    name,
    selectedColor,
    icon,
    conditions,
    matchCondition,
    transformRef,
  }) !== JSON.stringify({
    modalTab: initial.modalTab,
    name: editingBin?.name || '',
    selectedColor: editingBin?.color || '#3b82f6',
    icon: editingBin ? formatEmojiIcon(editingBin.icon) : '📂',
    conditions: initial.conditions,
    matchCondition: initial.matchCondition,
    transformRef: initialTransformRef.current,
  });

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="bin-modal-title"
      isDirty={isDirty}
      panelClassName="bin-modal-card theme-panel w-full max-w-xl max-h-[90vh] rounded-2xl border shadow-2xl overflow-hidden flex flex-col font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading id="bin-modal-title" title={editingBin ? 'Edit Bin' : 'New Bin'} description="Choose how clips enter this Bin and what happens next." icon={<Folder />} />
        </AppDialogHeader>
        <form onSubmit={handleSubmit} className="flex min-h-0 flex-1 flex-col">
          <AppDialogBody className="space-y-4 text-xs">
            <div className="flex justify-center">
              <div className="flex theme-surface p-1 rounded-xl border space-x-1">
                <button
                  type="button"
                  onClick={() => setModalTab('bin')}
                  className={`settings-tab px-4 py-1.5 rounded-lg text-xs font-semibold transition-none border border-transparent ${modalTab === 'bin' ? 'is-active' : ''}`}
                >
                  Manual
                </button>
                <button
                  type="button"
                  onClick={() => setModalTab('smart')}
                  className={`settings-tab px-4 py-1.5 rounded-lg text-xs font-semibold transition-none border border-transparent ${modalTab === 'smart' ? 'is-active' : ''}`}
                >
                  Smart
                </button>
              </div>
            </div>
          {/* Name Field */}
          <div className="flex items-center space-x-3">
            <label className={`w-14 text-right font-semibold flex-shrink-0 ${errors.name ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>Name:</label>
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
              className={`flex-1 theme-input rounded-xl border px-3 py-2 text-xs focus:outline-none font-medium transition-colors ${errors.name ? 'form-field-error' : 'form-field-valid'}`}
              autoFocus
            />
          </div>

          {/* Color Palette Picker Row */}
          <div className="flex items-center space-x-3">
            <label className={`w-14 text-right font-semibold flex-shrink-0 ${errors.color ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>Color:</label>
            <div className={`flex items-center space-x-2 p-1 rounded-xl border border-transparent transition-colors ${errors.color ? 'form-field-error' : ''}`}>
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
                      ? 'bin-color-selected scale-110'
                      : 'opacity-80 hover:opacity-100'
                  }`}
                  title={c.label}
                />
              ))}
            </div>
          </div>

          {/* Single Emoji Icon Selector */}
          <div className="flex items-center space-x-3">
            <label className={`w-14 text-right font-semibold flex-shrink-0 ${errors.icon ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>Icon:</label>
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
                className={`w-16 theme-input emoji-input-picker rounded-xl border py-1.5 text-center font-mono text-lg focus:outline-none shadow-inner cursor-pointer select-none transition-colors ${errors.icon ? 'form-field-error' : 'form-field-valid'}`}
                title="Open Emoji Picker"
              />
              <span className="text-[11px] theme-text-muted">
                Click input to open Emoji picker <kbd className="font-mono text-[10px] px-1.5 py-0.5 rounded theme-badge border">⌘Control+Space</kbd>
              </span>
            </div>
          </div>

          {/* Smart Bin Multi-Condition Builder */}
          {modalTab === 'smart' && (
            <div className="p-4 theme-surface rounded-2xl border space-y-3">
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
                    className="theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                  >
                    <option value="source_app">Source App</option>
                    <option value="content_type">Content Type</option>
                    <option value="contains">Text Content</option>
                  </select>

                  {/* Operator Dropdown */}
                  <select
                    value={c.operator}
                    onChange={(e) => handleUpdateCondition(c.id, { operator: e.target.value as any })}
                    className="theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                  >
                    <option value="is">is</option>
                    <option value="contains">contains</option>
                  </select>

                  {/* Dynamic Value Dropdown / Input */}
                  {c.target === 'source_app' ? (
                    <select
                      value={c.value}
                      onChange={(e) => handleUpdateCondition(c.id, { value: e.target.value })}
                      className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none truncate"
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
                      className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
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
                      className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
                    />
                  )}

                  {/* Active + / - Condition Buttons */}
                  <div className="flex items-center space-x-1">
                    <button
                      type="button"
                      onClick={() => handleRemoveCondition(c.id)}
                      disabled={conditions.length <= 1}
                      className={`theme-icon-button p-1.5 rounded border transition-[background-color,border-color,color,transform] ${
                        conditions.length <= 1
                          ? 'opacity-40 cursor-not-allowed'
                          : 'hover:scale-105 active:scale-95'
                      }`}
                      title="Remove Condition"
                    >
                      <Minus className="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      onClick={handleAddCondition}
                      className="theme-icon-button p-1.5 rounded border transition-[background-color,border-color,color,transform] hover:scale-105 active:scale-95"
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
                  className="theme-input form-field-valid border rounded-lg px-3 py-1 text-xs font-semibold focus:outline-none"
                >
                  <option value="any">any</option>
                  <option value="all">all</option>
                </select>
                <span>conditions</span>
              </div>
            </div>
          )}

          <div className="theme-surface space-y-2 rounded-2xl border p-4">
            <div>
              <span className="block text-xs font-semibold theme-text-main">When a clip enters this Bin</span>
              <p className="mt-0.5 text-[10px] theme-text-muted">Run one saved Transform. Its plan decides whether work stays local or uses connected intelligence.</p>
            </div>
            <MenuSelect
              value={transformRef}
              options={[
                { value: '', label: 'Do not transform clips' },
                ...transforms.map((transform) => ({ value: transform.stableRef, label: transform.name })),
              ]}
              onChange={setTransformRef}
              label="Transform clips entering this Bin"
              className="w-full"
            />
          </div>

          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose}>Cancel</AppDialogButton>
            <AppDialogButton type="submit" variant="primary">Save</AppDialogButton>
          </AppDialogFooter>
        </form>
      </>}
    </AppDialog>
  );
};
