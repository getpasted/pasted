import React, { useState } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  BookOpen,
  Terminal,
  Keyboard,
  Shield,
  Trash2,
  Sliders,
  ChevronRight,
  Copy,
  Check,
  Zap,
  Info,
  Command,
  Download,
} from 'lucide-react';

export const HelpView: React.FC = () => {
  const [activeSubTab, setActiveSubTab] = useState<'cli' | 'hotkeys' | 'autopause' | 'trash' | 'filters'>('cli');
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
      {/* Header Bar */}
      <div className="theme-toolbar h-[60px] border-b backdrop-blur-md px-6 flex items-center justify-between shrink-0 no-drag">
        <div className="flex items-center space-x-2.5">
          <BookOpen className="w-5 h-5 text-cyan-400" />
          <h2 className="text-sm font-bold text-gray-100 uppercase tracking-wider">
            Documentation
          </h2>
        </div>
        <div className="px-3 py-1 rounded-full bg-cyan-950/80 border border-cyan-800/60 text-cyan-300 text-xs font-mono font-semibold">
          v1.0.0 Pro Edition
        </div>
      </div>

      {/* Subpage Navigation & Content Container */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sub-Tab Sidebar Navigation */}
        <div className="theme-subtle-surface w-56 border-r p-3 space-y-1 shrink-0 overflow-y-auto">
          <button
            onClick={() => setActiveSubTab('cli')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold transition-all cursor-pointer border ${
              activeSubTab === 'cli'
                ? 'is-selected bg-cyan-500/20 text-cyan-300 border-cyan-500/40 shadow-sm'
                : 'text-gray-400 hover:text-white hover:bg-white/5'
            }`}
          >
            <div className="flex items-center space-x-2.5">
              <Terminal className="w-4 h-4 text-cyan-400" />
              <span>CLI Commands</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('hotkeys')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold transition-all cursor-pointer border ${
              activeSubTab === 'hotkeys'
                ? 'is-selected bg-cyan-500/20 text-cyan-300 border-cyan-500/40 shadow-sm'
                : 'text-gray-400 hover:text-white hover:bg-white/5'
            }`}
          >
            <div className="flex items-center space-x-2.5">
              <Keyboard className="w-4 h-4 text-emerald-400" />
              <span>Hotkeys & Modifiers</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('autopause')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold transition-all cursor-pointer border ${
              activeSubTab === 'autopause'
                ? 'is-selected bg-cyan-500/20 text-cyan-300 border-cyan-500/40 shadow-sm'
                : 'text-gray-400 hover:text-white hover:bg-white/5'
            }`}
          >
            <div className="flex items-center space-x-2.5">
              <Shield className="w-4 h-4 text-amber-400" />
              <span>Auto-Pause & Privacy</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('trash')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold transition-all cursor-pointer border ${
              activeSubTab === 'trash'
                ? 'is-selected bg-cyan-500/20 text-cyan-300 border-cyan-500/40 shadow-sm'
                : 'text-gray-400 hover:text-white hover:bg-white/5'
            }`}
          >
            <div className="flex items-center space-x-2.5">
              <Trash2 className="w-4 h-4 text-rose-400" />
              <span>Soft Trash Protection</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>

          <button
            onClick={() => setActiveSubTab('filters')}
            className={`help-topic-button w-full flex items-center justify-between px-3 py-2 text-xs font-semibold transition-all cursor-pointer border ${
              activeSubTab === 'filters'
                ? 'is-selected bg-cyan-500/20 text-cyan-300 border-cyan-500/40 shadow-sm'
                : 'text-gray-400 hover:text-white hover:bg-white/5'
            }`}
          >
            <div className="flex items-center space-x-2.5">
              <Sliders className="w-4 h-4 text-purple-400" />
              <span>Smart Filters</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 opacity-60" />
          </button>
        </div>

        {/* Right Detail Subpage Content */}
        <div className="flex-1 p-6 overflow-y-auto space-y-6">
          {activeSubTab === 'cli' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="text-lg font-bold text-gray-100 flex items-center space-x-2">
                  <Terminal className="w-5 h-5 text-cyan-400" />
                  <span>Pasted Terminal CLI Tool (`pasted-cli`)</span>
                </h3>
                <p className="text-xs text-gray-400 mt-1">
                  Pasted includes a standalone native command-line tool allowing terminal power users to pipe data into Pasted history, list clips, search from shell, or clear history.
                </p>
              </div>

              {/* PATH Installation Box */}
              <div className="p-4 rounded-xl bg-cyan-950/30 border border-cyan-500/30 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2 text-xs font-bold text-cyan-300">
                    <Download className="w-4 h-4 text-cyan-400" />
                    <span>Install CLI to $PATH</span>
                  </div>
                  <button
                    onClick={handleInstallCli}
                    className="flex items-center space-x-1.5 px-3 py-1.5 bg-cyan-500 hover:bg-cyan-400 text-black rounded-lg text-xs font-bold transition-all cursor-pointer shadow-sm"
                  >
                    <Download className="w-3.5 h-3.5" />
                    <span>1-Click Symlink to ~/.local/bin</span>
                  </button>
                </div>

                {installStatus && (
                  <div className="p-2.5 rounded-lg bg-black/60 border border-cyan-500/40 text-xs font-mono text-cyan-300">
                    {installStatus}
                  </div>
                )}

                <div className="text-xs text-gray-300 space-y-1">
                  <p className="font-semibold text-gray-200">Manual $PATH Setup Options:</p>
                  <div className="p-2.5 rounded-lg bg-black/70 border border-gray-800 font-mono text-[11px] text-gray-300 space-y-1">
                    <div className="text-emerald-400"># Symlink bundled macOS app executable to /usr/local/bin</div>
                    <div>$ sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted-cli /usr/local/bin/pasted-cli</div>
                    <div className="text-emerald-400 pt-1"># Or add alias in ~/.zshrc or ~/.bashrc</div>
                    <div>alias pasted-cli="/Applications/Pasted.app/Contents/MacOS/pasted-cli"</div>
                  </div>
                </div>
              </div>

              {/* CLI Command 1: Copy / Pipe */}
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-cyan-300 font-mono">1. Save text or pipe stdin into Pasted history</span>
                  <button
                    onClick={() => handleCopyCode('echo "Log data" | pasted-cli copy')}
                    className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-800"
                    title="Copy command"
                  >
                    {copiedCmd === 'echo "Log data" | pasted-cli copy' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="p-3 rounded-lg bg-black/60 border border-gray-800 font-mono text-xs text-gray-300">
                  <div className="text-emerald-400"># Direct string argument</div>
                  <div>$ pasted-cli copy "Hello from Terminal!"</div>
                  <div className="text-emerald-400 mt-2"># Pipe file or command stdout directly into Pasted</div>
                  <div>$ cat server.log | pasted-cli copy</div>
                </div>
              </div>

              {/* CLI Command 2: List */}
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-cyan-300 font-mono">2. List recent clipboard items</span>
                  <button
                    onClick={() => handleCopyCode('pasted-cli list 10')}
                    className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-800"
                    title="Copy command"
                  >
                    {copiedCmd === 'pasted-cli list 10' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="p-3 rounded-lg bg-black/60 border border-gray-800 font-mono text-xs text-gray-300">
                  <div className="text-emerald-400"># Output N recent clipboard items</div>
                  <div>$ pasted-cli list 15</div>
                </div>
              </div>

              {/* CLI Command 3: Search */}
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-cyan-300 font-mono">3. Keyword search clip database</span>
                  <button
                    onClick={() => handleCopyCode('pasted-cli search "api_key"')}
                    className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-800"
                    title="Copy command"
                  >
                    {copiedCmd === 'pasted-cli search "api_key"' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="p-3 rounded-lg bg-black/60 border border-gray-800 font-mono text-xs text-gray-300">
                  <div>$ pasted-cli search "https://"</div>
                </div>
              </div>
            </div>
          )}

          {activeSubTab === 'hotkeys' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="text-lg font-bold text-gray-100 flex items-center space-x-2">
                  <Keyboard className="w-5 h-5 text-emerald-400" />
                  <span>Pro Keyboard Shortcuts & Modifiers</span>
                </h3>
                <p className="text-xs text-gray-400 mt-1">
                  Hidden power shortcuts built for maximum speed and efficiency.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="flex items-center space-x-2 text-xs font-bold text-amber-300">
                    <Trash2 className="w-4 h-4 text-rose-400" />
                    <span>Option / Alt Key Permanent Delete</span>
                  </div>
                  <p className="text-xs text-gray-400">
                    Holding the <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">Option ⌥</kbd> key changes the Trash icon to a red <span className="text-red-400 font-bold">X</span> button to permanently purge items bypassing Trash.
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="flex items-center space-x-2 text-xs font-bold text-cyan-300">
                    <Command className="w-4 h-4 text-cyan-400" />
                    <span>Floating HUD Toggle</span>
                  </div>
                  <p className="text-xs text-gray-400">
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">⌥ Shift V</kbd> to pop open the transparent quick HUD next to your cursor.
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="flex items-center space-x-2 text-xs font-bold text-purple-300">
                    <Zap className="w-4 h-4 text-purple-400" />
                    <span>HUD Number Keys (1-9)</span>
                  </div>
                  <p className="text-xs text-gray-400">
                    Press numbers <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">1</kbd> through <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">9</kbd> inside the HUD to instantly paste items #1 to #9.
                  </p>
                </div>

              <div className="theme-panel p-4 rounded-xl border space-y-2">
                  <div className="flex items-center space-x-2 text-xs font-bold text-emerald-300">
                    <Info className="w-4 h-4 text-emerald-400" />
                    <span>Escape Key Dismiss</span>
                  </div>
                  <p className="text-xs text-gray-400">
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">Esc</kbd> to instantly dismiss the HUD or clear active search queries.
                  </p>
                </div>
              </div>
            </div>
          )}

          {activeSubTab === 'autopause' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="text-lg font-bold text-gray-100 flex items-center space-x-2">
                  <Shield className="w-5 h-5 text-amber-400" />
                  <span>Auto-Pause & Application Blacklisting</span>
                </h3>
                <p className="text-xs text-gray-400 mt-1">
                  Pasted protects your sensitive credentials by automatically pausing recording when focused on password managers.
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="text-xs font-bold text-amber-300">How Auto-Pause Works</h4>
                <p className="text-xs text-gray-300 leading-relaxed">
                  When switching active focus into applications like <strong>1Password</strong>, <strong>Keychain Access</strong>, <strong>Passwords</strong>, or <strong>Bitwarden</strong>, Pasted automatically pauses background recording and updates the Pause button state to glowing amber.
                </p>
                <p className="text-xs text-gray-400 leading-relaxed">
                  As soon as you switch back to allowed applications (e.g. VS Code, Chrome, Terminal), recording automatically resumes without losing any work!
                </p>
              </div>
            </div>
          )}

          {activeSubTab === 'trash' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="text-lg font-bold text-gray-100 flex items-center space-x-2">
                  <Trash2 className="w-5 h-5 text-rose-400" />
                  <span>Soft Trash Protection Layer</span>
                </h3>
                <p className="text-xs text-gray-400 mt-1">
                  Accidentally deleted a clip? Pasted provides a soft Trash protection layer so clips can be restored cleanly.
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="text-xs font-bold text-rose-300">Soft Deletion vs Hard Purging</h4>
                <ul className="text-xs text-gray-300 space-y-2 list-disc list-inside">
                  <li><strong>Normal Delete Click:</strong> Moves clip to the Trash tab. The sidebar badge updates instantly.</li>
                  <li><strong>Trash Tab Recovery:</strong> Click the <RotateCcwIcon /> Restore button to return items back to your history.</li>
                  <li><strong>Option / Alt Key Purge:</strong> Hold Option/Alt while clicking delete to permanently remove items immediately.</li>
                </ul>
              </div>
            </div>
          )}

          {activeSubTab === 'filters' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="text-lg font-bold text-gray-100 flex items-center space-x-2">
                  <Sliders className="w-5 h-5 text-purple-400" />
                  <span>Smart Filters & Text Transformations</span>
                </h3>
                <p className="text-xs text-gray-400 mt-1">
                  Transform copied text instantly with built-in case converters, sanitizers, and smart rules.
                </p>
              </div>

              <div className="theme-panel p-4 rounded-xl border space-y-3">
                <h4 className="text-xs font-bold text-purple-300">Available Transformations</h4>
                <div className="grid grid-cols-2 gap-2 text-xs font-mono text-gray-300">
                  <div className="p-2 rounded bg-gray-900 border border-gray-800">• UPPERCASE / lowercase</div>
                  <div className="p-2 rounded bg-gray-900 border border-gray-800">• Title Case / CamelCase</div>
                  <div className="p-2 rounded bg-gray-900 border border-gray-800">• Trim Whitespace</div>
                  <div className="p-2 rounded bg-gray-900 border border-gray-800">• Smart Punctuation</div>
                  <div className="p-2 rounded bg-gray-900 border border-gray-800">• URL Encode / Decode</div>
                  <div className="p-2 rounded bg-gray-900 border border-gray-800">• JSON Prettify</div>
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
  <span className="inline-block px-1 py-0.5 rounded bg-gray-800 border border-gray-700 text-gray-300 font-mono text-[10px]">
    Restore
  </span>
);
