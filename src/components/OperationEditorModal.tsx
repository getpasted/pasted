import React, { useEffect, useState } from 'react';
import { Operation } from '../types';
import { Braces, Play, Wrench } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { startWindowDrag } from '../utils/windowDrag';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { MenuSelect, type MenuSelectOption } from './MenuSelect';

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

function operationFormValues(operation: Operation | null) {
  let findPattern = '';
  let replacePattern = '';
  let aiInstructions = '';
  if (operation?.config) {
    try {
      const config = JSON.parse(operation.config);
      findPattern = config.pattern || '';
      replacePattern = config.replacement || '';
      aiInstructions = config.instructions || '';
    } catch {
      // The existing validation message remains responsible for malformed legacy data.
    }
  }
  return {
    name: operation?.name || '',
    opType: operation?.op_type || 'regex',
    category: operation?.category || 'Custom Operations',
    findPattern,
    replacePattern,
    aiInstructions,
  };
}

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
  const [saveError, setSaveError] = useState('');
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    const initial = operationFormValues(operation);
    setName(initial.name);
    setOpType(initial.opType);
    setCategory(initial.category);
    setFindPattern(initial.findPattern);
    setReplacePattern(initial.replacePattern);
    setAiInstructions(initial.aiInstructions);
    setSaveError('');

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

    setSaveError('');
    setIsSaving(true);
    try {
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
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  };

  if (!isOpen) return null;
  const isEditableKind = opType === 'regex' || opType === 'ai';
  const operationTypeOptions: MenuSelectOption[] = [
    { value: 'regex', label: 'Regex replacement · local and safe' },
    ...(!['regex', 'ai', 'cli', 'http'].includes(opType)
      ? [{ value: opType, label: `${opType} · legacy custom operation`, disabled: true }]
      : []),
    ...(opType === 'cli'
      ? [{ value: 'cli', label: 'Command or CLI · legacy and unavailable', disabled: true }]
      : []),
    ...(opType === 'http'
      ? [{ value: 'http', label: 'HTTP API · legacy and unavailable', disabled: true }]
      : []),
    { value: 'ai', label: 'Connected intelligence · priority and fallback' },
  ];
  const isDirty = JSON.stringify({ name, opType, category, findPattern, replacePattern, aiInstructions })
    !== JSON.stringify(operationFormValues(operation));

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="operation-editor-title"
      isDirty={isDirty}
      overlayClassName="p-6"
      panelClassName="filter-editor-card w-full max-w-2xl max-h-[90vh] border rounded-2xl flex flex-col overflow-hidden"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} onMouseDown={startWindowDrag}>
          <AppDialogHeading id="operation-editor-title" title={operation ? 'Edit Custom Operation' : 'New Custom Operation'} description="Custom Operations are yours to edit and reuse. Built-ins remain immutable." icon={<Wrench />} tone="info" />
        </AppDialogHeader>

        <AppDialogBody className="space-y-5">
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
            <MenuSelect
              value={opType}
              options={operationTypeOptions}
              onChange={setOpType}
              label="Operation engine type"
              className="w-full"
            />
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
          {saveError && <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{saveError}</div>}
        </AppDialogBody>

        <AppDialogFooter>
          <AppDialogButton onClick={requestClose}>Cancel</AppDialogButton>
          <AppDialogButton
            variant="primary"
            onClick={handleSave}
            disabled={isSaving || !name.trim() || !isEditableKind || (opType === 'ai' && !aiInstructions.trim())}
          >
            {isSaving ? 'Saving…' : operation ? 'Save Custom Operation' : 'Create Custom Operation'}
          </AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
};
