import React, { useEffect, useState } from 'react';
import { Operation } from '../types';
import { Braces, Play, Wrench, X } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { startWindowDrag } from '../utils/windowDrag';

interface OperationEditorModalProps {
  operation: Operation | null;
  isOpen: boolean;
  onClose: () => void;
  onSaveSuccess: () => void;
}

export const CATEGORIES = [
  'Custom Operations',
  'Text Cleanup',
  'Developer Tools',
  'Data Extraction',
  'Integrations',
];

export const OperationEditorModal: React.FC<OperationEditorModalProps> = ({
  operation,
  isOpen,
  onClose,
  onSaveSuccess,
}) => {
  const [name, setName] = useState('');
  const [opType, setOpType] = useState('regex');
  const [category, setCategory] = useState('Custom Operations');
  const [findPattern, setFindPattern] = useState('');
  const [replacePattern, setReplacePattern] = useState('');
  const [aiInstructions, setAiInstructions] = useState('');
  const [testInput, setTestInput] = useState('Hello Pasted Operation User! :)');
  const [testOutput, setTestOutput] = useState('');

  useEffect(() => {
    if (!isOpen) return;
    setName(operation?.name || '');
    setOpType(operation?.op_type || 'regex');
    setCategory(operation?.category || 'Custom Operations');
    setFindPattern('');
    setReplacePattern('');
    setAiInstructions('');

    if (operation?.op_type === 'regex' && operation.config) {
      try {
        const config = JSON.parse(operation.config);
        setFindPattern(config.pattern || '');
        setReplacePattern(config.replacement || '');
      } catch {
        setTestOutput('This operation has an invalid Regex configuration.');
      }
    }
    if (operation?.op_type === 'ai' && operation.config) {
      try {
        const config = JSON.parse(operation.config);
        setAiInstructions(config.instructions || '');
      } catch {
        setTestOutput('This operation has invalid connected-intelligence configuration.');
      }
    }
  }, [operation, isOpen]);

  useEffect(() => {
    if (!isOpen || opType !== 'regex') return;
    if (!testInput) {
      setTestOutput('');
      return;
    }

    const timer = window.setTimeout(async () => {
      try {
        const output = await invoke<string>('transform_text', {
          input: testInput,
          filterType: 'regex',
          config: JSON.stringify({ pattern: findPattern, replacement: replacePattern }),
        });
        setTestOutput(output);
      } catch (error) {
        setTestOutput(`Error: ${error}`);
      }
    }, 80);
    return () => window.clearTimeout(timer);
  }, [findPattern, isOpen, opType, replacePattern, testInput]);

  const handleSave = async () => {
    if (!name.trim() || !['regex', 'ai'].includes(opType)) return;
    if (opType === 'ai' && !aiInstructions.trim()) return;
    const config = opType === 'regex'
      ? JSON.stringify({ pattern: findPattern, replacement: replacePattern })
      : JSON.stringify({ instructions: aiInstructions.trim(), connectionId: null });

    if (operation) {
      await invoke('update_operation', {
        id: operation.id,
        name: name.trim(),
        opType,
        config,
        category,
      });
    } else {
      await invoke('create_operation', {
        name: name.trim(),
        opType,
        config,
        category,
      });
    }
    onSaveSuccess();
    onClose();
  };

  if (!isOpen) return null;
  const isEditableKind = opType === 'regex' || opType === 'ai';

  return (
    <div className="app-dialog-overlay fixed inset-0 flex items-center justify-center p-6 animate-in fade-in duration-150">
      <div className="filter-editor-card w-full max-w-2xl max-h-[90vh] border rounded-2xl flex flex-col overflow-hidden">
        <div onMouseDown={startWindowDrag} className="filter-editor-header px-6 py-4 border-b flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="theme-status-info p-2 rounded-xl border">
              <Wrench className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-bold theme-title">
                {operation ? 'Edit Custom Operation' : 'New Custom Operation'}
              </h3>
              <p className="text-xs theme-text-muted">
                Custom Operations are yours to edit and reuse. Built-ins remain immutable.
              </p>
            </div>
          </div>
          <button onClick={onClose} className="theme-icon-button p-2 border rounded-lg transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="filter-editor-body flex-1 overflow-y-auto p-6 space-y-5">
          <div className="grid grid-cols-2 gap-4 text-xs">
            <div>
              <label className="block font-semibold mb-1 theme-text-muted">Name</label>
              <input
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="e.g. Redact phone numbers"
                className="w-full border rounded-xl p-2.5 focus:outline-none font-medium theme-input"
                autoFocus
              />
            </div>
            <div>
              <label className="block font-semibold mb-1 theme-text-muted">Category</label>
              <input
                value={category}
                onChange={(event) => setCategory(event.target.value)}
                list="custom-operation-categories"
                className="w-full border rounded-xl p-2.5 focus:outline-none font-medium theme-input"
              />
              <datalist id="custom-operation-categories">
                {CATEGORIES.map((value) => <option key={value} value={value} />)}
              </datalist>
            </div>
          </div>

          <div className="text-xs">
            <label className="block font-semibold mb-1 theme-text-muted">Runs with</label>
            <select
              value={opType}
              onChange={(event) => setOpType(event.target.value)}
              className="w-full border rounded-xl p-2.5 focus:outline-none font-medium theme-input"
            >
              <option value="regex">Regex replacement · local and safe</option>
              {opType !== 'regex' && <option value={opType}>{opType} · legacy custom operation</option>}
              <option value="cli" disabled>Command or CLI · sandbox coming next</option>
              <option value="http" disabled>HTTP API · coming later</option>
              <option value="ai">Connected intelligence · priority and fallback</option>
            </select>
            <p className="theme-text-subtle text-[10px] mt-1.5">
              This chooses how a custom Operation runs; built-in transformations are maintained by Pasted and are not duplicated here.
            </p>
          </div>

          {opType === 'regex' ? (
            <>
              <div className="grid grid-cols-2 gap-3 text-xs">
                <div>
                  <label className="block mb-1 theme-text-muted">Find pattern</label>
                  <textarea
                    value={findPattern}
                    onChange={(event) => setFindPattern(event.target.value)}
                    placeholder="e.g. \\b\\d{3}-\\d{3}-\\d{4}\\b"
                    className="w-full h-20 border rounded-xl p-2.5 font-mono focus:outline-none theme-input"
                  />
                </div>
                <div>
                  <label className="block mb-1 theme-text-muted">Replace with</label>
                  <textarea
                    value={replacePattern}
                    onChange={(event) => setReplacePattern(event.target.value)}
                    placeholder="e.g. [REDACTED] or $1"
                    className="w-full h-20 border rounded-xl p-2.5 font-mono focus:outline-none theme-input"
                  />
                </div>
              </div>

              <div className="filter-sandbox-card p-4 rounded-2xl border space-y-3 shadow-inner">
                <div className="theme-status-info-text text-xs font-semibold uppercase tracking-wider flex items-center space-x-1.5">
                  <Play className="w-3.5 h-3.5" />
                  <span>Safe local preview</span>
                </div>
                <div className="grid grid-cols-2 gap-3 text-xs font-mono">
                  <textarea
                    value={testInput}
                    onChange={(event) => setTestInput(event.target.value)}
                    className="w-full h-20 border rounded-xl p-2.5 focus:outline-none theme-input"
                  />
                  <div className="filter-sandbox-output w-full h-20 border rounded-xl p-2.5 overflow-y-auto whitespace-pre-wrap theme-input">
                    {testOutput || 'Output will appear here…'}
                  </div>
                </div>
              </div>
            </>
          ) : opType === 'ai' ? (
            <div className="space-y-3 text-xs">
              <div>
                <label className="block mb-1 theme-text-muted">Instructions</label>
                <textarea
                  value={aiInstructions}
                  onChange={(event) => setAiInstructions(event.target.value)}
                  placeholder="For example: Rewrite this as concise, well-structured Markdown while preserving every fact and URL."
                  className="theme-input min-h-32 w-full resize-y rounded-xl border p-3 focus:outline-none"
                />
              </div>
              <div className="theme-card-idle flex items-start gap-3 rounded-xl border p-4">
                <Braces className="mt-0.5 h-4 w-4 theme-text-muted" />
                <p className="theme-text-muted">
                  Runs through the first enabled compatible Connection. The Operation stores instructions and routing—not credentials.
                </p>
              </div>
            </div>
          ) : (
            <div className="theme-card-idle border rounded-xl p-4 flex items-start gap-3 text-xs">
              <Braces className="w-4 h-4 mt-0.5 theme-text-muted" />
              <p className="theme-text-muted">
                This legacy executor is preserved, but editing and execution stay disabled until its sandbox, permissions, timeouts, and output limits are available.
              </p>
            </div>
          )}
        </div>

        <div className="filter-editor-footer px-6 py-4 border-t flex items-center justify-end space-x-3">
          <button onClick={onClose} className="filter-modal-cancel-btn px-4 py-2 rounded-xl text-xs font-medium transition-colors">
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={!name.trim() || !isEditableKind || (opType === 'ai' && !aiInstructions.trim())}
            className="filter-modal-ok-btn px-5 py-2 rounded-xl text-xs font-bold shadow-lg transition-[background-color,border-color,color,transform] active:scale-95 disabled:opacity-40"
          >
            {operation ? 'Save Custom Operation' : 'Create Custom Operation'}
          </button>
        </div>
      </div>
    </div>
  );
};
