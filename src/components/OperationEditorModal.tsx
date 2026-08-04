import React, { useState, useEffect } from 'react';
import { Operation } from '../types';
import { Wrench, X, Play } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { startWindowDrag } from '../utils/windowDrag';

interface OperationEditorModalProps {
  operation: Operation | null;
  isOpen: boolean;
  onClose: () => void;
  onSaveSuccess: () => void;
}

export const CATEGORIES = [
  'Cleaners & Sanitizers',
  'Case Transformations',
  'Smart Formatting',
  'Data Extraction',
  'Line Operations',
  'Structure & Tags',
  'Encodings & Decodings',
  'Advanced & Shell Scripts',
  'Custom Operations',
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
  const [shellCommand, setShellCommand] = useState('');
  const [tagName, setTagName] = useState('code');
  const [testInput, setTestInput] = useState('Hello Pasted Operation User! :)');
  const [testOutput, setTestOutput] = useState('');

  useEffect(() => {
    if (!isOpen) return;

    if (operation) {
      setName(operation.name);
      setOpType(operation.op_type);
      setCategory(operation.category || 'Custom Operations');
      setFindPattern('');
      setReplacePattern('');
      setShellCommand('');

      if (operation.config) {
        try {
          const cfg = JSON.parse(operation.config);
          if (cfg.pattern !== undefined) {
            setFindPattern(cfg.pattern || '');
            setReplacePattern(cfg.replacement || '');
          }
        } catch {
          if (operation.op_type === 'wrap_tags') {
            setTagName(operation.config);
          } else {
            setShellCommand(operation.config);
          }
        }
      } else if (operation.op_type === 'wrap_tags') {
        setTagName('code');
      }
    } else {
      setName('');
      setOpType('regex');
      setCategory('Custom Operations');
      setFindPattern('');
      setReplacePattern('');
      setShellCommand('');
      setTagName('code');
    }
  }, [operation, isOpen]);

  // Live test preview
  useEffect(() => {
    if (!isOpen) return;
    runLiveTest();
  }, [opType, findPattern, replacePattern, shellCommand, tagName, testInput, isOpen]);

  const runLiveTest = async () => {
    if (!testInput) {
      setTestOutput('');
      return;
    }
    try {
      let config: string | null = null;
      if (opType === 'regex') {
        config = JSON.stringify({ pattern: findPattern, replacement: replacePattern });
      } else if (opType === 'shell_script') {
        config = shellCommand;
      } else if (opType === 'wrap_tags') {
        config = tagName;
      }

      const res = await invoke<string>('transform_text', {
        input: testInput,
        filterType: opType,
        config,
      });
      setTestOutput(res);
    } catch (e) {
      setTestOutput(`Error: ${e}`);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    try {
      let config: string | null = null;
      if (opType === 'regex') {
        config = JSON.stringify({ pattern: findPattern, replacement: replacePattern });
      } else if (opType === 'wrap_tags') {
        config = tagName;
      } else if (shellCommand) {
        config = shellCommand;
      }

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
    } catch (e) {
      console.error(e);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="app-dialog-overlay fixed inset-0 flex items-center justify-center p-6 animate-in fade-in duration-150">
      <div className="filter-editor-card w-full max-w-2xl max-h-[90vh] border rounded-2xl flex flex-col overflow-hidden">
        {/* Header */}
        <div onMouseDown={startWindowDrag} className="filter-editor-header px-6 py-4 border-b flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="theme-status-info p-2 rounded-xl border">
              <Wrench className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-bold theme-title">
                {operation ? 'Edit Operation' : 'New Operation'}
              </h3>
              <p className="text-xs theme-text-muted">
                Build reusable operations for your operation library and filter pipelines.
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="theme-icon-button p-2 border rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="filter-editor-body flex-1 overflow-y-auto p-6 space-y-5">
          <div className="grid grid-cols-2 gap-4 text-xs">
            <div>
              <label className="block font-semibold mb-1 theme-text-muted">Operation Name:</label>
              <input
                type="text"
                placeholder="e.g. Redact Sensitive Phone Numbers"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full border rounded-xl p-2.5 focus:outline-none font-medium theme-input"
                autoFocus
              />
            </div>
            <div>
              <label className="block font-semibold mb-1 theme-text-muted">Category:</label>
              <input
                type="text"
                list="category-options"
                placeholder="Choose or type custom category..."
                value={category}
                onChange={(e) => setCategory(e.target.value)}
                className="w-full border rounded-xl p-2.5 focus:outline-none font-medium theme-input"
              />
              <datalist id="category-options">
                {CATEGORIES.map((cat) => (
                  <option key={cat} value={cat} />
                ))}
              </datalist>
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold mb-1 theme-text-muted">
              Operation Engine Type:
            </label>
            <select
              value={
                [
                  'regex',
                  'shell_script',
                  'wrap_tags',
                  'clean_url_tracking',
                  'strip_html',
                  'strip_markdown',
                  'strip_emojis',
                  'smileys_to_emoji',
                  'trim',
                  'strip_newlines',
                  'smart_punctuation',
                  'straighten_punctuation',
                  'uppercase',
                  'lowercase',
                  'titlecase',
                  'sentence_case',
                  'camelcase',
                  'snakecase',
                  'kebabcase',
                  'constant_case',
                  'alternating_case',
                  'json_format',
                  'json_minify',
                  'extract_urls',
                  'extract_emails',
                  'extract_phones',
                  'extract_ips',
                  'extract_numbers',
                  'sort_lines_asc',
                  'sort_lines_desc',
                  'sort_by_length',
                  'dedupe_lines',
                  'reverse_lines',
                  'strip_empty_lines',
                  'number_lines',
                  'quote_text',
                  'html_encode',
                  'html_decode',
                  'hex_encode',
                  'hex_decode',
                  'url_encode',
                  'url_decode',
                  'base64_encode',
                  'base64_decode',
                ].includes(opType)
                  ? opType
                  : 'custom_identifier'
              }
              onChange={(e) => {
                const val = e.target.value;
                if (val === 'custom_identifier') {
                  setOpType('custom_engine_key');
                } else {
                  setOpType(val);
                }
              }}
              className="w-full border rounded-xl p-2.5 text-xs focus:outline-none font-medium theme-input"
            >
              <optgroup label="Core Engines">
                <option value="regex">Find & Replace (Regex Pattern)</option>
                <option value="shell_script">Shell Script Command (sh -c / stdin -&gt; stdout)</option>
                <option value="wrap_tags">Wrap in HTML Tags</option>
              </optgroup>
              <optgroup label="Cleaners & Sanitizers">
                <option value="clean_url_tracking">Clean URL Tracking (Strip UTM / fbclid)</option>
                <option value="strip_html">Plain Text / Strip HTML</option>
                <option value="strip_markdown">Strip Markdown Formatting</option>
                <option value="strip_emojis">Emoji Remover</option>
                <option value="smileys_to_emoji">Convert Smileys to Emoji</option>
                <option value="trim">Trim Whitespace</option>
                <option value="strip_newlines">Strip Newlines</option>
              </optgroup>
              <optgroup label="Smart Formatting">
                <option value="smart_punctuation">Smart Punctuation (“ ” — …)</option>
                <option value="straighten_punctuation">Straighten Punctuation (" ' -)</option>
              </optgroup>
              <optgroup label="Case Transformations">
                <option value="uppercase">UPPERCASE</option>
                <option value="lowercase">lowercase</option>
                <option value="titlecase">Title Case</option>
                <option value="sentence_case">Sentence case</option>
                <option value="camelcase">camelCase</option>
                <option value="snakecase">snake_case</option>
                <option value="kebabcase">kebab-case</option>
                <option value="constant_case">CONSTANT_CASE</option>
                <option value="alternating_case">aLtErNaTiNg cAsE</option>
              </optgroup>
              <optgroup label="Structure & Formatting">
                <option value="json_format">Format JSON (Pretty Print)</option>
                <option value="json_minify">Minify JSON (Compact)</option>
              </optgroup>
              <optgroup label="Data Extraction">
                <option value="extract_urls">Extract URLs</option>
                <option value="extract_emails">Extract Emails</option>
                <option value="extract_phones">Extract Phone Numbers</option>
                <option value="extract_ips">Extract IP Addresses</option>
                <option value="extract_numbers">Extract Numbers</option>
              </optgroup>
              <optgroup label="Line Operations">
                <option value="sort_lines_asc">Sort Lines (A-Z)</option>
                <option value="sort_lines_desc">Sort Lines (Z-A)</option>
                <option value="sort_by_length">Sort Lines (By Length)</option>
                <option value="dedupe_lines">Deduplicate Lines</option>
                <option value="reverse_lines">Reverse Lines</option>
                <option value="strip_empty_lines">Strip Empty Lines</option>
                <option value="number_lines">Number Lines (1. 2. 3.)</option>
                <option value="quote_text">Quote Text (&gt; )</option>
              </optgroup>
              <optgroup label="Encodings & Decodings">
                <option value="html_encode">HTML Entity Encode</option>
                <option value="html_decode">HTML Entity Decode</option>
                <option value="hex_encode">Hex Encode</option>
                <option value="hex_decode">Hex Decode</option>
                <option value="url_encode">URL Encode</option>
                <option value="url_decode">URL Decode</option>
                <option value="base64_encode">Base64 Encode</option>
                <option value="base64_decode">Base64 Decode</option>
              </optgroup>
              <optgroup label="Custom Registration">
                <option value="custom_identifier">+ Custom Operation Type Identifier...</option>
              </optgroup>
            </select>
          </div>

          {![
            'regex',
            'shell_script',
            'wrap_tags',
            'clean_url_tracking',
            'strip_html',
            'strip_markdown',
            'strip_emojis',
            'smileys_to_emoji',
            'trim',
            'strip_newlines',
            'smart_punctuation',
            'straighten_punctuation',
            'uppercase',
            'lowercase',
            'titlecase',
            'sentence_case',
            'camelcase',
            'snakecase',
            'kebabcase',
            'constant_case',
            'alternating_case',
            'json_format',
            'json_minify',
            'extract_urls',
            'extract_emails',
            'extract_phones',
            'extract_ips',
            'extract_numbers',
            'sort_lines_asc',
            'sort_lines_desc',
            'sort_by_length',
            'dedupe_lines',
            'reverse_lines',
            'strip_empty_lines',
            'number_lines',
            'quote_text',
            'html_encode',
            'html_decode',
            'hex_encode',
            'hex_decode',
            'url_encode',
            'url_decode',
            'base64_encode',
            'base64_decode',
          ].includes(opType) && (
            <div className="text-xs">
              <label className="block mb-1 theme-text-muted">Custom Type Identifier Key:</label>
              <input
                type="text"
                placeholder="e.g. my_custom_engine_key"
                value={opType}
                onChange={(e) => setOpType(e.target.value)}
                className="w-full border rounded-xl p-2.5 font-mono focus:outline-none theme-input"
              />
            </div>
          )}

          {(opType === 'regex' || findPattern) && (
            <div className="grid grid-cols-2 gap-3 text-xs">
              <div>
                <label className="block mb-1 theme-text-muted">Find Pattern (Regex):</label>
                <textarea
                  placeholder="Regex pattern e.g. \b\d{3}-\d{3}-\d{4}\b"
                  value={findPattern}
                  onChange={(e) => setFindPattern(e.target.value)}
                  className="w-full h-20 border rounded-xl p-2.5 font-mono focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="block mb-1 theme-text-muted">Replace With:</label>
                <textarea
                  placeholder="Replacement string e.g. [REDACTED] or $1"
                  value={replacePattern}
                  onChange={(e) => setReplacePattern(e.target.value)}
                  className="w-full h-20 border rounded-xl p-2.5 font-mono focus:outline-none theme-input"
                />
              </div>
            </div>
          )}

          {(opType === 'shell_script' || opType === 'custom' || shellCommand) && opType !== 'regex' && !findPattern && (
            <div className="text-xs">
              <label className="block mb-1 theme-text-muted">External Shell Command / Pipe Script (stdin -&gt; stdout):</label>
              <textarea
                placeholder='e.g. tr "a-z" "A-Z" or tesseract stdin stdout or python3 -c "import sys; print(sys.stdin.read())"'
                value={shellCommand}
                onChange={(e) => setShellCommand(e.target.value)}
                className="w-full h-20 border rounded-xl p-2.5 font-mono focus:outline-none theme-input"
              />
              <p className="theme-text-subtle text-[10px] mt-1">
                Pipes clipboard text through any Bash, Python, JQ, Tesseract OCR, or CLI binary.
              </p>
            </div>
          )}

          {/* Sticky Interactive Sandbox */}
          <div className="filter-sandbox-card p-4 rounded-2xl border space-y-3 shadow-inner">
            <div className="flex items-center justify-between text-xs">
              <span className="theme-status-info-text font-semibold uppercase tracking-wider flex items-center space-x-1.5">
                <Play className="w-3.5 h-3.5" />
                <span>Live Tester Sandbox</span>
              </span>
            </div>
            <div className="grid grid-cols-2 gap-3 text-xs font-mono">
              <div>
                <label className="block mb-1 font-sans theme-text-muted">Input:</label>
                <textarea
                  value={testInput}
                  onChange={(e) => setTestInput(e.target.value)}
                  className="w-full h-20 border rounded-xl p-2.5 focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="filter-sandbox-output-label block mb-1 font-sans font-semibold">Output:</label>
                <div className="filter-sandbox-output w-full h-20 border rounded-xl p-2.5 overflow-y-auto whitespace-pre-wrap theme-input">
                  {testOutput || 'Filtered output result will appear here...'}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="filter-editor-footer px-6 py-4 border-t flex items-center justify-end space-x-3">
          <button
            type="button"
            onClick={onClose}
            className="filter-modal-cancel-btn px-4 py-2 rounded-xl text-xs font-medium transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            className="filter-modal-ok-btn px-5 py-2 rounded-xl text-xs font-bold shadow-lg transition-[background-color,border-color,color,transform] active:scale-95"
          >
            {operation ? 'Save Operation' : 'Create Operation'}
          </button>
        </div>
      </div>
    </div>
  );
};
