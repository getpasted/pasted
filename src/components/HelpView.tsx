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
  Bell,
  Radar,
  ScanText,
  History,
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
      { usage: 'pasted import <alfred|pastebot|pasta|paste|copyclip|maccy|flycut> [path] [--json]', description: 'Merge text history from another clipboard manager, skipping duplicates.' },
      { usage: 'pasted retention [--count N] [--days N] [--trash-count N] [--trash-days N] [--log-count N] [--log-days N] [--json]', description: 'Read or update History, Trash, and Activity count and age policies.' },
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
      { usage: 'pasted transform list [--json]', description: 'List saved and manually built Transforms.' },
      { usage: 'pasted transform get <ref> [--json]', description: 'Inspect one canonical Transform definition.' },
      { usage: 'pasted transform create --name NAME (--plan-json JSON | --steps-json JSON) [--json]', description: 'Create a planned or manually built Transform.' },
      { usage: 'pasted transform update <ref> [options] [--json]', description: 'Update a Transform without changing its stable reference or authoring form.' },
      { usage: 'pasted transform duplicate <ref> [--name NAME] [--json]', description: 'Duplicate a Transform with a new stable reference.' },
      { usage: 'pasted transform delete <ref> [--json]', description: 'Delete a Transform; existing clip revisions remain unchanged.' },
      { usage: 'pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--replace]', description: 'Preview a Transform, or replace a clip while preserving a revision.' },
      { usage: 'pasted operation list [--json]', description: 'Inspect built-in and custom Operations.' },
      { usage: 'pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]', description: 'Run one Operation through the shared executor.' },
    ],
  },
  {
    title: 'Detection',
    commands: [
      { usage: 'pasted registry list [--kind detector|operation|transform] [--all] [--json]', description: 'Inspect shared lifecycle and input/output contracts for processing assets.' },
      { usage: 'pasted registry enable|disable --kind detector|operation --ref REF [--json]', description: 'Change the shared enabled state using a stable processing-asset reference.' },
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
      { usage: 'pasted licenses [--json]', description: 'Show the bundled open-source component inventory and legal notices.' },
      { usage: 'pasted library location [--json]', description: 'Show the active SQLite library location.' },
      { usage: 'pasted library move <folder> [--json]', description: 'Move the library safely after quitting the Pasted app.' },
      { usage: 'pasted library default [--json]', description: 'Return the SQLite library to Pasted’s native default location.' },
      { usage: 'pasted ocr status [--json]', description: 'Inspect OCR backfill progress.' },
      { usage: 'pasted ocr scan', description: 'Process eligible images that have not been OCR’d.' },
      { usage: 'pasted reset --yes [--json]', description: 'Reset all Pasted data and preferences. This is destructive.' },
    ],
  },
] as const;

export type HelpTopic = 'getting-started' | 'cli' | 'hotkeys' | 'autopause' | 'trash' | 'detection' | 'pipelines';

interface HelpTopicDefinition {
  id: HelpTopic;
  label: string;
  icon: LucideIcon;
  iconClassName: string;
}

const HELP_TOPICS: HelpTopicDefinition[] = [
  { id: 'getting-started', label: 'Getting Started', icon: BookOpen, iconClassName: 'theme-status-info-text' },
  { id: 'hotkeys', label: 'Shortcuts & HUD', icon: Keyboard, iconClassName: 'theme-status-success-text' },
  { id: 'autopause', label: 'Privacy & Capture', icon: Shield, iconClassName: 'theme-status-warning-text' },
  { id: 'trash', label: 'Deletion & Recovery', icon: Trash2, iconClassName: 'theme-status-danger-text' },
  { id: 'detection', label: 'Detection & OCR', icon: Radar, iconClassName: 'theme-status-info-text' },
  { id: 'pipelines', label: 'Transformations', icon: Workflow, iconClassName: 'theme-status-info-text' },
  { id: 'cli', label: 'CLI Commands', icon: Terminal, iconClassName: 'theme-status-info-text' },
];

interface HelpViewProps {
  requestedTopic?: HelpTopic;
  navigationKey?: number;
}

export const HelpView: React.FC<HelpViewProps> = ({ requestedTopic, navigationKey }) => {
  const { showToast } = useToast();
  const [activeSubTab, setActiveSubTab] = useState<HelpTopic>('getting-started');
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
          {activeSubTab === 'getting-started' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title flex items-center space-x-2 text-lg font-bold">
                  <BookOpen className="h-5 w-5 theme-status-info-text" />
                  <span>Getting Started with Pasted</span>
                </h3>
                <p className="theme-text-muted mt-1 text-xs">
                  Pasted keeps a local history of the text, images, screenshots, PDFs, and files you copy while it is running.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-title text-xs font-bold">The main window</h4>
                  <ol className="theme-text-main list-inside list-decimal space-y-2 text-xs leading-relaxed">
                    <li>Choose History, a collection, or a Bin from the left sidebar.</li>
                    <li>Select a clip from the middle column.</li>
                    <li>Preview, copy, organize, or transform it in the right column.</li>
                  </ol>
                  <p className="theme-text-muted text-xs">Drag the column dividers to resize the layout. Pasted remembers your window and column sizes.</p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-title text-xs font-bold">First useful actions</h4>
                  <ul className="theme-text-main list-inside list-disc space-y-2 text-xs leading-relaxed">
                    <li>Copy normally in another app to add an item to History.</li>
                    <li>Use Search to find clip content, Types, Sources, notes, or status.</li>
                    <li>Right-click a clip for Queue, Pin, Protect, Note, Bin, Transform, and Trash actions.</li>
                    <li>Open Settings → Functionality to choose the Simple or Full experience.</li>
                  </ul>
                </section>
              </div>

              <div className="theme-status-warning rounded-xl border p-4">
                <h4 className="text-xs font-bold">Features normally hide without deleting data</h4>
                <p className="mt-1 text-xs leading-relaxed">
                  Disabling a feature usually hides its interface and stops new behavior while preserving existing data. Important exceptions are shown beside the setting: disabling Trash makes new deletions permanent, and disabling Revision History makes new edits and Transform replacements irreversible.
                </p>
              </div>
            </div>
          )}

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
                  <span>Shortcuts & HUD</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Use the default shortcuts below, or change and disable them under Settings → Hotkeys.
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
                    <span>Open HUD</span>
                  </div>
                  <p className="theme-text-muted text-xs">
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">⌥ Shift V</kbd> to open the compact clipboard window near your pointer. Use arrow keys to select and Enter to paste.
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
                  <span>Privacy & Capture</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Control which applications Pasted records and how it confirms a capture without sending clipboard contents away from your device.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-warning-text text-xs font-bold">Auto-pause and blacklist</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Pasted starts with common password managers such as <strong>1Password</strong>, <strong>Keychain Access</strong>, <strong>Passwords</strong>, and <strong>Bitwarden</strong> on its blacklist. When an excluded app is focused, capture pauses and the Pause control turns amber.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Capture resumes when focus returns to an allowed app. Add applications or choose whether they block text, images, and shortcuts under Settings → Blacklist.
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-info-text flex items-center gap-2 text-xs font-bold">
                    <Bell className="h-4 w-4" />
                    <span>Capture feedback</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Settings → Notifications controls quiet capture confirmations, skipped-capture messages, optional clip previews, dismissal timing, and screen position. Disabling Notifications does not disable clipboard capture.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Feedback is rendered locally by Pasted and does not send copied text, images, file names, or paths through system notification services. Optional previews can still be visible on screen, so disable them before screen sharing when appropriate.
                  </p>
                </section>
              </div>
            </div>
          )}

          {activeSubTab === 'trash' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Trash2 className="w-5 h-5 theme-status-danger-text" />
                  <span>Deletion & Recovery</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Understand which actions are recoverable before removing or changing important clips.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-danger-text text-xs font-bold">Trash and permanent deletion</h4>
                  <ul className="theme-text-main list-inside list-disc space-y-2 text-xs">
                    <li><strong>Normal deletion:</strong> moves an eligible clip to Trash while Trash is enabled.</li>
                    <li><strong>Restore:</strong> use the <RotateCcwIcon /> action in Trash to return a clip to active History.</li>
                    <li><strong>Permanent deletion:</strong> hold Option/Alt while deleting, purge from Trash, or disable Trash.</li>
                    <li><strong>Protection:</strong> protected clips resist deletion and automatic retention until unprotected.</li>
                  </ul>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-info-text flex items-center gap-2 text-xs font-bold">
                    <History className="h-4 w-4" />
                    <span>Revisions and backups</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Revision History saves restorable snapshots before content-changing edits and Transform replacements. Disabling it preserves old revisions but makes new changes irreversible.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Use Settings → Storage to export a backup before major changes or Factory Reset. Factory Reset permanently removes the local library and preferences after confirmation.
                  </p>
                </section>
              </div>
            </div>
          )}

          {activeSubTab === 'detection' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title flex items-center space-x-2 text-lg font-bold">
                  <Radar className="h-5 w-5 theme-status-info-text" />
                  <span>Detection & OCR</span>
                </h3>
                <p className="theme-text-muted mt-1 text-xs">
                  Local detectors classify text into useful Types, while OCR makes captured images searchable on supported macOS systems.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-info-text text-xs font-bold">Content Detection</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Enabled detectors run locally in priority order; the lowest number runs first. A detector uses one or more regular expressions and may add a validator to reduce false positives. Use Settings → Detection to test samples, manage Types, and restore shipped defaults.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Editing a detector affects new text clips. <strong>Rescan History</strong> explicitly reapplies the current detector order to existing text and can change Types, Smart Bin membership, and sensitive-content masking. Images and files are left unchanged.
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <ScanText className="h-4 w-4" />
                    <span>Optical Character Recognition</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    OCR uses Apple Vision on macOS to extract searchable text from captured images and screenshots. It does not replace the original image.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Disabling OCR cancels background work and discards late results while preserving completed text. Re-enabling it resumes eligible backfill. Check progress under Settings → Diagnostics or with <code>pasted ocr status --json</code>.
                  </p>
                </section>
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

              <div className="theme-subtle-surface rounded-xl border p-4">
                <h4 className="text-xs font-bold">Advanced Transformation Tools</h4>
                <p className="theme-text-muted mt-1 text-xs">
                  Operations are deterministic building blocks for reusable Transforms. Manually built Transforms retain their existing pipeline identifiers for shortcuts, automations, backups, and command-line compatibility.
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
