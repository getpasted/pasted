import React, { useEffect, useState } from 'react';
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
  type LucideIcon,
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { useToast } from './ToastProvider';

const CLI_SYMLINK_COMMAND = 'sudo ln -s /Applications/Pasted.app/Contents/MacOS/pasted /usr/local/bin/pasted';
const CLI_ALIAS_COMMAND = 'alias pasted="/Applications/Pasted.app/Contents/MacOS/pasted"';

const CLI_COMMAND_GROUPS = [
  {
    title: 'History',
    commands: [
      { usage: 'pasted copy "Hello"', description: 'Save a text clip. Omit the argument to read stdin.' },
      { usage: 'cat server.log | pasted copy', description: 'Pipe bounded text into Pasted history.' },
      { usage: 'pasted list [limit]', description: 'List recent active clips; defaults to 10.' },
      { usage: 'pasted search [query] [--type TYPE] [--source APP] [--json]', description: 'Search active clips or reproduce Type and Source views.' },
      { usage: 'pasted clear', description: 'Permanently remove unpinned, unprotected clips.' },
    ],
  },
  {
    title: 'Clip actions',
    commands: [
      { usage: 'pasted clip get <id> [--json]', description: 'Inspect one clip and its metadata.' },
      { usage: 'pasted clip pin|unpin <id>... [--json]', description: 'Set pin state explicitly for one or more clips.' },
      { usage: 'pasted clip protect|unprotect <id>... [--json]', description: 'Set protection explicitly for one or more clips.' },
      { usage: 'pasted clip trash|restore <id>... [--json]', description: 'Move clips into or out of Trash.' },
      { usage: 'pasted clip assign <bin-id|none> <id>... [--json]', description: 'Assign clips to one manual Bin, or remove their manual Bin.' },
    ],
  },
  {
    title: 'Bins & Transforms',
    commands: [
      { usage: 'pasted bin list [--json]', description: 'List Bins, counts, and saved ordering.' },
      { usage: 'pasted bin clips <bin-id> [--json]', description: 'List a Bin’s clips in persistent order.' },
      { usage: 'pasted bin order <bin-id> <clip-id>... [--json]', description: 'Replace a Bin’s complete saved clip order.' },
      { usage: 'pasted transform list', description: 'List reusable saved Transforms.' },
      { usage: 'pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--replace]', description: 'Preview a Transform, or replace a clip while preserving a revision.' },
      { usage: 'pasted operation list [--json]', description: 'Inspect experimental built-in and custom Operations.' },
      { usage: 'pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]', description: 'Run one experimental Operation through the shared executor.' },
      { usage: 'pasted pipeline list [--json]', description: 'Inspect experimental deterministic Pipelines.' },
      { usage: 'pasted pipeline run <ref> [--text TEXT | --clip ID | --stdin] [--json]', description: 'Run one experimental Pipeline through the shared executor.' },
    ],
  },
  {
    title: 'Detection',
    commands: [
      { usage: 'pasted type list [--all] [--json]', description: 'List registered content Types and their display metadata.' },
      { usage: 'pasted type create --id ID --name NAME [--icon ICON] [--group GROUP] [--json]', description: 'Create a custom Type with a stable ID.' },
      { usage: 'pasted type update <id> [options] [--json]', description: 'Customize a Type’s name, icon, or group without changing its ID.' },
      { usage: 'pasted type archive|restore <id>', description: 'Archive or restore a custom Type while preserving historical clips.' },
      { usage: 'pasted type restore-defaults', description: 'Restore built-in Type names, icons, and groups.' },
      { usage: 'pasted type group-list [--all] [--json]', description: 'List registered Content Type Groups.' },
      { usage: 'pasted type group-create --id ID --name NAME [--order NUMBER]', description: 'Create a reusable custom Type Group.' },
      { usage: 'pasted type group-update <id> [options] [--json]', description: 'Rename or reorder a Type Group.' },
      { usage: 'pasted type group-archive|group-restore <id>', description: 'Archive an empty custom Group or restore it.' },
      { usage: 'pasted type group-delete <id>', description: 'Permanently delete an empty custom Group.' },
      { usage: 'pasted detector list [--json]', description: 'List editable detectors in effective priority order.' },
      { usage: 'pasted detector create --name NAME --type TYPE --regex REGEX [--json]', description: 'Create a custom detector.' },
      { usage: 'pasted detector update <id> [options] [--json]', description: 'Edit an existing shipped or custom detector.' },
      { usage: 'pasted detector delete <id>', description: 'Delete a detector; shipped defaults remain recoverable.' },
      { usage: 'pasted detector restore-defaults', description: 'Restore shipped detectors without removing custom entries.' },
      { usage: 'pasted detector rescan --yes [--json]', description: 'Explicitly reclassify existing text clips with the current enabled detector order.' },
    ],
  },
  {
    title: 'Maintenance',
    commands: [
      { usage: 'pasted diagnostics [--json]', description: 'Show installation, signing, paths, and runtime details.' },
      { usage: 'pasted library location [--json]', description: 'Show the active SQLite library location.' },
      { usage: 'pasted library move <folder> [--json]', description: 'Move the library safely after quitting the Pasted app.' },
      { usage: 'pasted library default [--json]', description: 'Return the SQLite library to Pasted’s native default location.' },
      { usage: 'pasted ocr status [--json]', description: 'Inspect OCR backfill progress.' },
      { usage: 'pasted ocr scan', description: 'Process eligible images that have not been OCR’d.' },
      { usage: 'pasted reset --yes [--json]', description: 'Reset all Pasted data and preferences. This is destructive.' },
    ],
  },
] as const;

export type HelpTopic = 'cli' | 'hotkeys' | 'autopause' | 'trash' | 'pipelines';

interface HelpTopicDefinition {
  id: HelpTopic;
  label: string;
  icon: LucideIcon;
  iconClassName: string;
}

const HELP_TOPICS: HelpTopicDefinition[] = [
  { id: 'cli', label: 'CLI Commands', icon: Terminal, iconClassName: 'theme-status-info-text' },
  { id: 'hotkeys', label: 'Hotkeys & Modifiers', icon: Keyboard, iconClassName: 'theme-status-success-text' },
  { id: 'autopause', label: 'Auto-Pause & Privacy', icon: Shield, iconClassName: 'theme-status-warning-text' },
  { id: 'trash', label: 'Soft Trash Protection', icon: Trash2, iconClassName: 'theme-status-danger-text' },
  { id: 'pipelines', label: 'Transformations', icon: Workflow, iconClassName: 'theme-status-info-text' },
];

interface HelpViewProps {
  requestedTopic?: HelpTopic;
  navigationKey?: number;
}

export const HelpView: React.FC<HelpViewProps> = ({ requestedTopic, navigationKey }) => {
  const { showToast } = useToast();
  const [activeSubTab, setActiveSubTab] = useState<HelpTopic>('cli');
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);

  useEffect(() => {
    if (requestedTopic) setActiveSubTab(requestedTopic);
  }, [navigationKey, requestedTopic]);

  const handleCopyCode = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCmd(code);
    setTimeout(() => setCopiedCmd(null), 1500);
  };

  const handleInstallCli = async () => {
    try {
      const res = await invoke<string>('install_cli_to_path');
      showToast({ tone: 'success', message: res });
    } catch (e: any) {
      showToast({ tone: 'error', message: String(e), durationMs: 8000 });
    }
  };

  return (
    <div className="tools-page help-page flex-1 font-sans h-screen flex flex-col overflow-hidden select-none">
      <ToolPageHeader
        icon={<BookOpen className="w-4 h-4" />}
        title="Documentation"
      />

      {/* Subpage Navigation & Content Container */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sub-Tab Sidebar Navigation */}
        <div className="help-topic-nav theme-subtle-surface">
          {HELP_TOPICS.map(({ id, label, icon: Icon, iconClassName }) => {
            const isSelected = activeSubTab === id;

            return (
              <button
                key={id}
                type="button"
                onClick={() => setActiveSubTab(id)}
                className={`help-topic-button ${isSelected ? 'is-selected' : ''}`}
                aria-current={isSelected ? 'page' : undefined}
              >
                <span className="help-topic-button__label">
                  <Icon className={iconClassName} />
                  <span>{label}</span>
                </span>
                <ChevronRight className="help-topic-button__chevron" aria-hidden="true" />
              </button>
            );
          })}
        </div>

        {/* Right Detail Subpage Content */}
        <div className="tools-scroll-region flex-1 p-6 overflow-y-auto space-y-6">
          {activeSubTab === 'cli' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Terminal className="w-5 h-5 theme-status-info-text" />
                  <span>Pasted Terminal CLI Tool (<code>pasted</code>)</span>
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
                    className="theme-primary-button ui-control-radius flex items-center space-x-1.5 px-3 py-1.5 border text-xs font-bold transition-colors cursor-pointer shadow-sm"
                  >
                    <Download className="w-3.5 h-3.5" />
                    <span>1-Click Symlink to ~/.local/bin</span>
                  </button>
                </div>

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
                          title="Copy Command"
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
                          title="Copy Alias"
                        >
                          {copiedCmd === CLI_ALIAS_COMMAND ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_ALIAS_COMMAND}</code>
                    </div>
                  </div>
                </div>
              </div>

              <div className="space-y-3">
                <div>
                  <h4 className="theme-title text-sm font-bold">Command reference</h4>
                  <p className="theme-text-muted mt-1 text-xs">
                    Commands that return records or mutation details support <code>--json</code> where shown. Disabled Features reject their related commands instead of silently changing data.
                  </p>
                </div>
                <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
                  {CLI_COMMAND_GROUPS.map((group) => (
                    <section key={group.title} className="theme-panel overflow-hidden rounded-xl border">
                      <h5 className="theme-section-label theme-divider border-b px-4 py-3 text-[11px] font-bold uppercase tracking-[0.12em]">
                        {group.title}
                      </h5>
                      <div className="theme-divide divide-y">
                        {group.commands.map((command) => (
                          <div key={command.usage} className="flex items-start gap-3 px-4 py-3">
                            <div className="min-w-0 flex-1">
                              <code className="selectable-text theme-status-info-text block select-text break-all font-mono text-[11px] font-semibold">
                                {command.usage}
                              </code>
                              <p className="theme-text-muted mt-1 text-xs leading-relaxed">{command.description}</p>
                            </div>
                            <button
                              type="button"
                              onClick={() => handleCopyCode(command.usage)}
                              className="theme-icon-button shrink-0 rounded border p-1.5"
                              title="Copy Command"
                            >
                              {copiedCmd === command.usage ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                            </button>
                          </div>
                        ))}
                      </div>
                    </section>
                  ))}
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
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">Esc</kbd> to instantly dismiss the HUD or close an open menu.
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
                  Describe what you want once, save it as a Transform, then reuse it wherever text enters or leaves Pasted.
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

              <div className="theme-status-warning rounded-xl border p-4">
                <h4 className="text-xs font-bold">Experimental Advanced Tools</h4>
                <p className="theme-text-muted mt-1 text-xs">
                  Operations and legacy Pipelines provide deterministic building blocks for advanced workflows. Their saved identifiers are stable, while their editor and command surface may evolve after 1.0.
                </p>
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
