import React, { useState } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  BookOpen,
  Terminal,
  Keyboard,
  Shield,
  Trash2,
  Workflow,
  ChevronRight,
  Copy,
  Check,
  Zap,
  Info,
  Command,
  Download,
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';

const CLI_SYMLINK_COMMAND = 'sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted-cli /usr/local/bin/pasted-cli';
const CLI_ALIAS_COMMAND = 'alias pasted-cli="/Applications/Pasted.app/Contents/MacOS/pasted-cli"';

export const HelpView: React.FC = () => {
  const [activeSubTab, setActiveSubTab] = useState<'cli' | 'hotkeys' | 'autopause' | 'trash' | 'pipelines'>('cli');
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);
  const [installStatus, setInstallStatus] = useState<string | null>(null);

  const handleCopyCode = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCmd(code);
    setTimeout(() => setCopiedCmd(null), 1500);
  };

  const handleInstallCli = async () => {
    try {
      const res = await invoke<string>('install_cli_to_path');
      setInstallStatus(res);
    } catch (e: any) {
      setInstallStatus(`Error: ${e}`);
    }
  };

  return (
    <div className="tools-page help-page flex-1 font-sans h-screen flex flex-col overflow-hidden select-none">
      <ToolPageHeader
        icon={<BookOpen className="w-4 h-4" />}
        title="Documentation"
        actions={(
          <div className="theme-badge px-3 py-1 rounded-full border text-xs font-mono font-semibold">
            v1.0.0 Pro Edition
          </div>
        )}
      />

      {/* Subpage Navigation & Content Container */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sub-Tab Sidebar Navigation */}
        <div className="theme-subtle-surface w-56 border-r p-3 space-y-1 shrink-0 overflow-y-auto">
          <button
            onClick={() => setActiveSubTab('cli')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold cursor-pointer border ${activeSubTab === 'cli' ? 'is-selected' : ''}`}
          >
            <div className="flex items-center space-x-2.5">
              <Terminal className="w-4 h-4 theme-status-info-text" />
              <span>CLI Commands</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('hotkeys')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold cursor-pointer border ${activeSubTab === 'hotkeys' ? 'is-selected' : ''}`}
          >
            <div className="flex items-center space-x-2.5">
              <Keyboard className="w-4 h-4 theme-status-success-text" />
              <span>Hotkeys & Modifiers</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('autopause')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold cursor-pointer border ${activeSubTab === 'autopause' ? 'is-selected' : ''}`}
          >
            <div className="flex items-center space-x-2.5">
              <Shield className="w-4 h-4 theme-status-warning-text" />
              <span>Auto-Pause & Privacy</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('trash')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold cursor-pointer border ${activeSubTab === 'trash' ? 'is-selected' : ''}`}
          >
            <div className="flex items-center space-x-2.5">
              <Trash2 className="w-4 h-4 theme-status-danger-text" />
              <span>Soft Trash Protection</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('pipelines')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold cursor-pointer border ${activeSubTab === 'pipelines' ? 'is-selected' : ''}`}
          >
            <div className="flex items-center space-x-2.5">
              <Workflow className="w-4 h-4 theme-status-info-text" />
              <span>Transformations</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>
        </div>

        {/* Right Detail Subpage Content */}
        <div className="tools-scroll-region flex-1 p-6 overflow-y-auto space-y-6">
          {activeSubTab === 'cli' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Terminal className="w-5 h-5 theme-status-info-text" />
                  <span>Pasted Terminal CLI Tool (`pasted-cli`)</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Pasted includes a standalone native command-line tool allowing terminal power users to pipe data into Pasted history, list clips, search from shell, or clear history.
                </p>
              </div>

              {/* PATH Installation Box */}
              <div className="theme-status-info p-4 rounded-xl border space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2 text-xs font-bold">
                    <Download className="w-4 h-4" />
                    <span>Install CLI to $PATH</span>
                  </div>
                  <button
                    onClick={handleInstallCli}
                    className="theme-primary-button flex items-center space-x-1.5 px-3 py-1.5 border rounded-lg text-xs font-bold transition-colors cursor-pointer shadow-sm"
                  >
                    <Download className="w-3.5 h-3.5" />
                    <span>1-Click Symlink to ~/.local/bin</span>
                  </button>
                </div>

                {installStatus && (
                  <div className="theme-code-surface p-2.5 rounded-lg border text-xs font-mono">
                    {installStatus}
                  </div>
                )}

                <div className="theme-text-main space-y-2 text-xs">
                  <p className="font-semibold theme-title">Manual $PATH setup:</p>
                  <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">Symlink in /usr/local/bin</span>
                        <button
                          type="button"
                          onClick={() => handleCopyCode(CLI_SYMLINK_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title="Copy symlink command"
                        >
                          {copiedCmd === CLI_SYMLINK_COMMAND ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_SYMLINK_COMMAND}</code>
                    </div>

                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">Shell alias</span>
                        <button
                          type="button"
                          onClick={() => handleCopyCode(CLI_ALIAS_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title="Copy shell alias"
                        >
                          {copiedCmd === CLI_ALIAS_COMMAND ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_ALIAS_COMMAND}</code>
                    </div>
                  </div>
                </div>
              </div>

              {/* CLI Command 1: Copy / Pipe */}
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                <div className="flex items-center justify-between">
                  <span className="theme-status-info-text text-xs font-bold font-mono">1. Save text or pipe stdin into Pasted history</span>
                  <button
                    onClick={() => handleCopyCode('echo "Log data" | pasted-cli copy')}
                    className="theme-icon-button p-1 rounded border"
                    title="Copy command"
                  >
                    {copiedCmd === 'echo "Log data" | pasted-cli copy' ? <Check className="w-3.5 h-3.5 theme-status-success-text" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="theme-code-surface p-3 rounded-lg border font-mono text-xs">
                  <div className="theme-status-success-text"># Direct string argument</div>
                  <div>$ pasted-cli copy "Hello from Terminal!"</div>
                  <div className="theme-status-success-text mt-2"># Pipe file or command stdout directly into Pasted</div>
                  <div>$ cat server.log | pasted-cli copy</div>
                </div>
              </div>

              {/* CLI Command 2: List */}
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                <div className="flex items-center justify-between">
                  <span className="theme-status-info-text text-xs font-bold font-mono">2. List recent clipboard items</span>
                  <button
                    onClick={() => handleCopyCode('pasted-cli list 10')}
                    className="theme-icon-button p-1 rounded border"
                    title="Copy command"
                  >
                    {copiedCmd === 'pasted-cli list 10' ? <Check className="w-3.5 h-3.5 theme-status-success-text" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="theme-code-surface p-3 rounded-lg border font-mono text-xs">
                  <div className="theme-status-success-text"># Output N recent clipboard items</div>
                  <div>$ pasted-cli list 15</div>
                </div>
              </div>

              {/* CLI Command 3: Search */}
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                <div className="flex items-center justify-between">
                  <span className="theme-status-info-text text-xs font-bold font-mono">3. Keyword search clip database</span>
                  <button
                    onClick={() => handleCopyCode('pasted-cli search "api_key"')}
                    className="theme-icon-button p-1 rounded border"
                    title="Copy command"
                  >
                    {copiedCmd === 'pasted-cli search "api_key"' ? <Check className="w-3.5 h-3.5 theme-status-success-text" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="theme-code-surface p-3 rounded-lg border font-mono text-xs">
                  <div>$ pasted-cli search "https://"</div>
                </div>
              </div>
            </div>
          )}

          {activeSubTab === 'hotkeys' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Keyboard className="w-5 h-5 theme-status-success-text" />
                  <span>Pro Keyboard Shortcuts & Modifiers</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Hidden power shortcuts built for maximum speed and efficiency.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-warning-text flex items-center space-x-2 text-xs font-bold">
                    <Trash2 className="w-4 h-4 theme-status-danger-text" />
                    <span>Option / Alt Key Permanent Delete</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    Holding the <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">Option ⌥</kbd> key changes the Trash icon to a red <span className="theme-status-danger-text font-bold">X</span> button to permanently purge items bypassing Trash.
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-info-text flex items-center space-x-2 text-xs font-bold">
                    <Command className="w-4 h-4" />
                    <span>Floating HUD Toggle</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">⌥ Shift V</kbd> to pop open the transparent quick HUD next to your cursor.
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-info-text flex items-center space-x-2 text-xs font-bold">
                    <Zap className="w-4 h-4" />
                    <span>HUD Number Keys (1-9)</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    Press numbers <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">1</kbd> through <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">9</kbd> inside the HUD to instantly paste items #1 to #9.
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="theme-status-success-text flex items-center space-x-2 text-xs font-bold">
                    <Info className="w-4 h-4" />
                    <span>Escape Key Dismiss</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">Esc</kbd> to instantly dismiss the HUD or clear active search queries.
                  </p>
                </div>
              </div>
            </div>
          )}

          {activeSubTab === 'autopause' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Shield className="w-5 h-5 theme-status-warning-text" />
                  <span>Auto-Pause & Application Blacklisting</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Pasted protects your sensitive credentials by automatically pausing recording when focused on password managers.
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="theme-status-warning-text text-xs font-bold">How Auto-Pause Works</h4>
                <p className="theme-text-main text-xs leading-relaxed">
                  When switching active focus into applications like <strong>1Password</strong>, <strong>Keychain Access</strong>, <strong>Passwords</strong>, or <strong>Bitwarden</strong>, Pasted automatically pauses background recording and updates the Pause button state to glowing amber.
                </p>
                <p className="theme-text-muted text-xs leading-relaxed">
                  As soon as you switch back to allowed applications (e.g. VS Code, Chrome, Terminal), recording automatically resumes without losing any work!
                </p>
              </div>
            </div>
          )}

          {activeSubTab === 'trash' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Trash2 className="w-5 h-5 theme-status-danger-text" />
                  <span>Soft Trash Protection Layer</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Accidentally deleted a clip? Pasted provides a soft Trash protection layer so clips can be restored cleanly.
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="theme-status-danger-text text-xs font-bold">Soft Deletion vs Hard Purging</h4>
                <ul className="theme-text-main text-xs space-y-2 list-disc list-inside">
                  <li><strong>Normal Delete Click:</strong> Moves clip to the Trash tab. The sidebar badge updates instantly.</li>
                  <li><strong>Trash Tab Recovery:</strong> Click the <RotateCcwIcon /> Restore button to return items back to your history.</li>
                  <li><strong>Option / Alt Key Purge:</strong> Hold Option/Alt while clicking delete to permanently remove items immediately.</li>
                </ul>
              </div>
            </div>
          )}

          {activeSubTab === 'pipelines' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Workflow className="w-5 h-5 theme-status-info-text" />
                  <span>Transformations</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Combine reusable Operations into Pipelines, then run them wherever text enters or leaves Pasted.
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="theme-status-info-text text-xs font-bold">Available Transformations</h4>
                <div className="theme-text-main grid grid-cols-2 gap-2 text-xs font-mono">
                  <div className="theme-code-surface p-2 rounded border">• UPPERCASE / lowercase</div>
                  <div className="theme-code-surface p-2 rounded border">• Title Case / CamelCase</div>
                  <div className="theme-code-surface p-2 rounded border">• Trim Whitespace</div>
                  <div className="theme-code-surface p-2 rounded border">• Smart Punctuation</div>
                  <div className="theme-code-surface p-2 rounded border">• URL Encode / Decode</div>
                  <div className="theme-code-surface p-2 rounded border">• JSON Prettify</div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const RotateCcwIcon = () => (
  <span className="theme-kbd inline-block px-1 py-0.5 rounded border font-mono text-[10px]">
    Restore
  </span>
);
