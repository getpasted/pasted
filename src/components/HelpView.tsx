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
  AudioLines,
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
      { usage: 'cat server.log | pasted copy', description: 'Pipe bounded text into clipboard history.' },
      { usage: 'pasted list [--limit N] [--offset N] [--bin ID | --pinned | --trash] [--json]', description: 'List a bounded page from History, Trash, a Bin, or pinned clips.' },
      { usage: 'pasted search [query] [--type TYPE] [--source APP] [--trash] [--limit N] [--offset N] [--json]', description: 'Search a bounded page of History or Trash with Content Type and Source filters.' },
      { usage: 'pasted import sources [--json]', description: 'List supported external-history sources and their detected locations.' },
      { usage: 'pasted import <alfred|pastebot|pasta|paste|copyclip|maccy|flycut> [path] [--json]', description: 'Merge text history from another clipboard manager, skipping duplicates.' },
      { usage: 'pasted retention [--count N] [--days N] [--trash-count N] [--trash-days N] [--log-count N] [--log-days N] [--revision-count N] [--json]', description: 'Read or update History, Trash, Activity, and revision retention.' },
      { usage: 'pasted settings list|get|set [arguments] [--json]', description: 'Inspect or change persisted application settings.' },
      { usage: 'pasted recording status|pause|resume [--json]', description: 'Control clipboard recording in the running app.' },
      { usage: 'pasted queue status|start|stop|add|remove|order|paste|paste-all [arguments] [--json]', description: 'Manage and run the live Copy Queue.' },
      { usage: 'pasted clear --yes [--json]', description: 'Permanently remove unpinned, unprotected clips.' },
    ],
  },
  {
    title: 'Clip actions',
    commands: [
      { usage: 'pasted clip get <id> [--json]', description: 'Inspect one clip and its metadata.' },
      { usage: 'pasted clip note <id> [--text TEXT | --clear | --stdin] [--json]', description: 'Set or clear a clip note.' },
      { usage: 'pasted clip revisions <id> [--limit N] [--offset N] [--json]', description: 'List retained clip revisions.' },
      { usage: 'pasted clip restore-revision <id> <revision-id> [--json]', description: 'Restore an earlier clip revision and its recorded organization.' },
      { usage: 'pasted clip provenance <id> [--json]', description: 'Inspect the Transform that produced the current clip content.' },
      { usage: 'pasted clip copy|paste <id> [--json]', description: 'Copy or paste a saved clip through the running app.' },
      { usage: 'pasted clip pin|unpin <id>... [--json]', description: 'Set pin state explicitly for one or more clips.' },
      { usage: 'pasted clip order-pinned <id>... [--json]', description: 'Replace the complete pinned-clip order.' },
      { usage: 'pasted clip protect|unprotect <id>... [--json]', description: 'Set protection explicitly for one or more clips.' },
      { usage: 'pasted clip trash|restore <id>... [--json]', description: 'Move clips into or out of Trash.' },
      { usage: 'pasted clip restore-all [--json]', description: 'Return every trashed clip to History.' },
      { usage: 'pasted clip purge <id>... --yes [--json]', description: 'Permanently delete unprotected clips.' },
      { usage: 'pasted clip empty-trash --yes [--json]', description: 'Permanently delete every unprotected clip in Trash.' },
      { usage: 'pasted clip export [path] [--format json|csv]', description: 'Export clips currently in History for external analysis.' },
      { usage: 'pasted clip import <path> [--format json|csv] [--json]', description: 'Preflight and merge clip records while skipping duplicates.' },
      { usage: 'pasted clip assign <bin-id|none> <id>... [--json]', description: 'Assign clips to one manual Bin, or remove their manual Bin.' },
    ],
  },
  {
    title: 'Bins and Transforms',
    commands: [
      { usage: 'pasted bin list [--json]', description: 'List Bins, counts, and saved ordering.' },
      { usage: 'pasted bin get <id> [--json]', description: 'Inspect one Bin and its attached Transform.' },
      { usage: 'pasted bin create --name NAME [options] [--json]', description: 'Create a manual or Smart Bin.' },
      { usage: 'pasted bin update <id> [options] [--json]', description: 'Update a Bin definition.' },
      { usage: 'pasted bin duplicate <id> [--name NAME] [--json]', description: 'Duplicate a Bin and its attached Transform.' },
      { usage: 'pasted bin delete <id> [--disposition keep|trash|move] [--json]', description: 'Delete a Bin with an explicit clip disposition.' },
      { usage: 'pasted bin clips <bin-id> [--json]', description: 'List a Bin’s clips in persistent order.' },
      { usage: 'pasted bin order <bin-id> <clip-id>... [--json]', description: 'Replace a Bin’s complete saved clip order.' },
      { usage: 'pasted bin transform <id> <transform-ref|none> [--json]', description: 'Set or clear a Bin’s default Transform.' },
      { usage: 'pasted bin shortcut <id> <shortcut|none> [--json]', description: 'Set or clear a Bin shortcut.' },
      { usage: 'pasted transform list [--json]', description: 'List saved and manually built Transforms.' },
      { usage: 'pasted transform get <ref> [--json]', description: 'Inspect one canonical Transform definition.' },
      { usage: 'pasted transform plan [--intent TEXT | --stdin] [--sample TEXT] [--json]', description: 'Draft a Transform plan from natural-language intent.' },
      { usage: 'pasted transform test --plan-json JSON [--text TEXT | --stdin] [--json]', description: 'Execute an unsaved Transform plan without changing a clip.' },
      { usage: 'pasted transform create --name NAME (--intent TEXT | --plan-json JSON | --steps-json JSON) [--json]', description: 'Create an intent-planned or manually built Transform.' },
      { usage: 'pasted transform update <ref> [options] [--json]', description: 'Update a Transform without changing its stable reference or authoring form.' },
      { usage: 'pasted transform duplicate <ref> [--name NAME] [--json]', description: 'Duplicate a Transform with a new stable reference.' },
      { usage: 'pasted transform delete <ref> [--json]', description: 'Delete a Transform; existing clip revisions remain unchanged.' },
      { usage: 'pasted transform run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]', description: 'Run a Transform in preview mode, or apply it to a clip while preserving a revision.' },
      { usage: 'pasted operation list [--json]', description: 'Inspect built-in and custom Operations.' },
      { usage: 'pasted operation get <ref> [--json]', description: 'Inspect one Operation definition.' },
      { usage: 'pasted operation create --name NAME --type TYPE [options] [--json]', description: 'Create an Operation.' },
      { usage: 'pasted operation update <ref> [options] [--json]', description: 'Update a custom Operation.' },
      { usage: 'pasted operation duplicate <ref> [--name NAME] [--json]', description: 'Duplicate an Operation with a new stable reference.' },
      { usage: 'pasted operation delete <ref> [--json]', description: 'Delete a custom Operation.' },
      { usage: 'pasted operation run <ref> [--text TEXT | --clip ID | --stdin] [--json]', description: 'Run one Operation through the shared executor.' },
      { usage: 'pasted connection list [--json]', description: 'List connected-intelligence providers in priority order.' },
      { usage: 'pasted connection get <id> [--json]', description: 'Inspect one Connection definition.' },
      { usage: 'pasted connection detect [--json]', description: 'Discover supported local intelligence providers.' },
      { usage: 'pasted connection create --name NAME --provider KIND [options] [--json]', description: 'Create a Connection using credential references only.' },
      { usage: 'pasted connection update <id> [options] [--json]', description: 'Update or enable a Connection.' },
      { usage: 'pasted connection delete <id> [--json]', description: 'Delete a Connection definition.' },
      { usage: 'pasted connection order <id>... [--json]', description: 'Replace Connection priority order.' },
    ],
  },
  {
    title: 'Content Analysis',
    commands: [
      { usage: 'pasted analyzer run [--text TEXT | --clip ID | --stdin] [--policy POLICY] [--extract] [--json]', description: 'Preview one versioned, content-free snapshot across the applicable Analysis passes.' },
      { usage: 'pasted registry list [--kind capture|inspector|extractor|classifier|suggestion|operation|transform] [--all] [--json]', description: 'Inspect shared lifecycle and input/output contracts for processing assets.' },
      { usage: 'pasted registry enable|disable --kind extractor|classifier|operation --ref REF [--json]', description: 'Change the shared enabled state using a stable processing-asset reference.' },
      { usage: 'pasted inspector list [--json]', description: 'List Inspectors, contracts, and system availability.' },
      { usage: 'pasted inspector get <ref> [--json]', description: 'Inspect one Inspector definition.' },
      { usage: 'pasted inspector run [--text TEXT | --clip ID | --stdin] [--apply] [--json]', description: 'Inspect content-free structure and live media metadata, or persist clip structure.' },
      { usage: 'pasted suggestion list [--json]', description: 'List Suggestions and their contracts.' },
      { usage: 'pasted suggestion get <ref> [--json]', description: 'Inspect one Suggestion definition.' },
      { usage: 'pasted suggestion run [--text TEXT | --clip ID | --stdin] [--json]', description: 'Suggest saved Transforms without changing content.' },
      { usage: 'pasted extractor list [--json]', description: 'List Extractors, contracts, and system availability.' },
      { usage: 'pasted extractor get <ref> [--json]', description: 'Inspect one Extractor definition.' },
      { usage: 'pasted extractor create [options] [--json]', description: 'Create an Extractor.' },
      { usage: 'pasted extractor update <ref> [options] [--json]', description: 'Update an Extractor definition.' },
      { usage: 'pasted extractor duplicate <ref> [--name NAME] [--json]', description: 'Duplicate an Extractor with a new stable reference.' },
      { usage: 'pasted extractor delete <ref> [--json]', description: 'Delete an Extractor; shipped defaults remain recoverable.' },
      { usage: 'pasted extractor run <ref> (--clip ID | --file PATH) [--apply] [--json]', description: 'Run an Extractor in preview mode, or apply its output to a clip.' },
      { usage: 'pasted extractor restore-defaults', description: 'Restore shipped Extractor settings.' },
      { usage: 'pasted type list [--all] [--json]', description: 'List registered Content Types and their display metadata.' },
      { usage: 'pasted type create --id ID --name NAME [--icon ICON] [--group GROUP] [--json]', description: 'Create a custom Content Type with a stable ID.' },
      { usage: 'pasted type update <id> [options] [--json]', description: 'Customize a Content Type’s name, icon, or group without changing its ID.' },
      { usage: 'pasted type archive|restore <id>', description: 'Archive or restore a custom Content Type while preserving historical clips.' },
      { usage: 'pasted type restore-defaults', description: 'Restore built-in Content Type names, icons, and groups.' },
      { usage: 'pasted type group-list [--all] [--json]', description: 'List registered Content Type Groups.' },
      { usage: 'pasted type group-create --id ID --name NAME [--order NUMBER]', description: 'Create a reusable custom Content Type Group.' },
      { usage: 'pasted type group-update <id> [options] [--json]', description: 'Rename or reorder a Content Type Group.' },
      { usage: 'pasted type group-archive|group-restore <id>', description: 'Archive an empty custom Group or restore it.' },
      { usage: 'pasted type group-delete <id>', description: 'Permanently delete an empty custom Group.' },
      { usage: 'pasted classifier list [--json]', description: 'List Classifiers in effective priority order.' },
      { usage: 'pasted classifier get <ref> [--json]', description: 'Inspect one Classifier definition.' },
      { usage: 'pasted classifier create --name NAME --type TYPE --regex REGEX [--json]', description: 'Create a Classifier.' },
      { usage: 'pasted classifier update <ref> [options] [--json]', description: 'Update a Classifier definition.' },
      { usage: 'pasted classifier duplicate <ref> [--name NAME] [--json]', description: 'Duplicate a Classifier with a new stable reference.' },
      { usage: 'pasted classifier delete <ref> [--json]', description: 'Delete a Classifier; shipped defaults remain recoverable.' },
      { usage: 'pasted classifier run <ref> [--text TEXT | --clip ID | --stdin] [--apply] [--json]', description: 'Run a Classifier in preview mode, or apply its matching Content Type to a clip.' },
      { usage: 'pasted classifier restore-defaults', description: 'Restore shipped Classifiers without removing custom entries.' },
      { usage: 'pasted classifier rescan --yes [--json]', description: 'Explicitly reclassify existing text clips with the current enabled Classifier order.' },
    ],
  },
  {
    title: 'Maintenance',
    commands: [
      { usage: 'pasted diagnostics [--json]', description: 'Show installation, signing, paths, and runtime details.' },
      { usage: 'pasted insights summary [--json]', description: 'Summarize Clip Types, File Formats, Content Types, sources, and daily activity.' },
      { usage: 'pasted licenses [--json]', description: 'Show the bundled open-source component inventory and legal notices.' },
      { usage: 'pasted database location [--json]', description: 'Show the active SQLite database location.' },
      { usage: 'pasted database move <folder> [--json]', description: 'Move the database safely after quitting.' },
      { usage: 'pasted database default [--json]', description: 'Return the SQLite database to its native default location.' },
      { usage: 'pasted transfer export <path.json> [--json]', description: 'Export history and organization as portable JSON.' },
      { usage: 'pasted transfer inspect <path.json> [--json]', description: 'Validate and summarize portable JSON without changing saved data.' },
      { usage: 'pasted transfer import <path.json> [--json]', description: 'Preflight and merge history and organization by stable identity and content hash.' },
      { usage: 'pasted backup create <path.pastedbackup> [--json]', description: 'Create a validated snapshot of every durable state store.' },
      { usage: 'pasted backup inspect <path.pastedbackup> [--json]', description: 'Validate a Full Backup and inspect its manifest without restoring it.' },
      { usage: 'pasted backup restore <path.pastedbackup> --yes [--json]', description: 'Replace the current state after creating a complete recovery backup.' },
      { usage: 'pasted ocr status [--json]', description: 'Inspect OCR backfill progress.' },
      { usage: 'pasted ocr scan [--clip ID] [--json]', description: 'Process eligible images or rescan one image clip.' },
      { usage: 'pasted ocr retry [--json]', description: 'Reset failed OCR attempts and process them again.' },
      { usage: 'pasted ocr cancel [--json]', description: 'Cancel OCR work in the running app.' },
      { usage: 'pasted reset --yes [--json]', description: 'Reset all data and preferences. This is destructive.' },
    ],
  },
  {
    title: 'Activity',
    commands: [
      { usage: 'pasted activity list [--limit N|--all] [--offset N] [--category VALUE] [--severity VALUE] [--event NAME] [--json]', description: 'List or filter a bounded page of retained Activity entries.' },
      { usage: 'pasted activity export [path] [--format json|csv]', description: 'Export all retained Activity entries for reporting.' },
      { usage: 'pasted activity import <path> [--format json|csv] [--json]', description: 'Merge inert Activity records without replaying their actions.' },
      { usage: 'pasted activity clear --yes [--json]', description: 'Permanently remove every retained Activity entry.' },
    ],
  },
] as const;

export type HelpTopic = 'getting-started' | 'cli' | 'shortcuts-hud' | 'privacy-capture' | 'deletion-recovery' | 'analysis' | 'transformations';

interface HelpTopicDefinition {
  id: HelpTopic;
  label: string;
  icon: LucideIcon;
  iconClassName: string;
}

const HELP_TOPICS: HelpTopicDefinition[] = [
  { id: 'getting-started', label: 'Getting Started', icon: BookOpen, iconClassName: 'theme-status-info-text' },
  { id: 'shortcuts-hud', label: 'Shortcuts and HUD', icon: Keyboard, iconClassName: 'theme-status-success-text' },
  { id: 'privacy-capture', label: 'Privacy and Capture', icon: Shield, iconClassName: 'theme-status-warning-text' },
  { id: 'deletion-recovery', label: 'Deletion and Recovery', icon: Trash2, iconClassName: 'theme-status-danger-text' },
  { id: 'analysis', label: 'Content Analysis', icon: Radar, iconClassName: 'theme-status-info-text' },
  { id: 'transformations', label: 'Transformations', icon: Workflow, iconClassName: 'theme-status-info-text' },
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
        title="Help"
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
                  <span>Getting Started</span>
                </h3>
                <p className="theme-text-muted mt-1 text-xs">
                  Local history includes copied text, images, screenshots, PDFs, and files captured while clipboard monitoring is active.
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
                  <p className="theme-text-muted text-xs">Drag the column dividers to resize the layout. Window and column sizes are remembered.</p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-title text-xs font-bold">First useful actions</h4>
                  <ul className="theme-text-main list-inside list-disc space-y-2 text-xs leading-relaxed">
                    <li>Copy normally in another app to add an item to History.</li>
                    <li>Use Search to find clip content, Content Types, Sources, notes, or status.</li>
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
                  <span>Terminal CLI (<code>pasted</code>)</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  The standalone native command-line tool can pipe data into clipboard history, list clips, search from a shell, or clear history.
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
                  <p className="font-semibold theme-title">Manual $PATH setup</p>
                  <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">Symlink in /usr/local/bin</span>
                        <button
                          type="button"
                          onClick={() => handleCopyCode(CLI_SYMLINK_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title="Copy command"
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
                          title="Copy alias"
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
                              title="Copy command"
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

          {activeSubTab === 'shortcuts-hud' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Keyboard className="w-5 h-5 theme-status-success-text" />
                  <span>Shortcuts and HUD</span>
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
                    Press <kbd className="theme-kbd px-1.5 py-0.5 rounded border font-mono text-[10px]">⌥ Shift V</kbd> to open the compact clipboard window near the pointer. Use arrow keys to select and Enter to paste.
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

          {activeSubTab === 'privacy-capture' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Shield className="w-5 h-5 theme-status-warning-text" />
                  <span>Privacy and Capture</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Control which applications are recorded and how captures are confirmed without sending clipboard contents off-device.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-warning-text text-xs font-bold">Auto-pause and app exclusions</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Common password managers such as <strong>1Password</strong>, <strong>Keychain Access</strong>, <strong>Passwords</strong>, and <strong>Bitwarden</strong> are excluded by default. Text, images, files, and hotkeys can be blocked independently.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Blocking every content kind presents as an automatic capture pause; partial rules skip only the selected kinds. Native Wayland sessions cannot identify the globally focused app, so App Exclusions cannot be enforced there.
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
                    Feedback is rendered locally and does not send copied text, images, file names, or paths through system notification services. Optional previews can still be visible on screen, so disable them before screen sharing when appropriate.
                  </p>
                </section>
              </div>
            </div>
          )}

          {activeSubTab === 'deletion-recovery' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Trash2 className="w-5 h-5 theme-status-danger-text" />
                  <span>Deletion and Recovery</span>
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
                    <li><strong>Restore:</strong> use the <RotateCcwIcon /> action in Trash to return a clip to History.</li>
                    <li><strong>Restore trashed clips:</strong> use Settings → General → Trash → Restore Trashed Clips to return every trashed clip to History.</li>
                    <li><strong>Permanent deletion:</strong> hold Option/Alt while deleting, purge from Trash, or disable Trash.</li>
                    <li><strong>Protection:</strong> protected clips resist deletion and automatic retention until unprotected.</li>
                  </ul>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-info-text flex items-center gap-2 text-xs font-bold">
                    <History className="h-4 w-4" />
                    <span>Revisions and Full Backups</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Revision History saves restorable snapshots before content-changing edits and Transform replacements. Disabling it preserves old revisions but makes new changes irreversible.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Use Settings → Storage to create a Full Backup before major changes or Factory Reset. Full Restore validates the backup and preserves the replaced state as a recovery backup before activation.
                  </p>
                </section>
              </div>
            </div>
          )}

          {activeSubTab === 'analysis' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title flex items-center space-x-2 text-lg font-bold">
                  <Radar className="h-5 w-5 theme-status-info-text" />
                  <span>Content Analysis</span>
                </h3>
                <p className="theme-text-muted mt-1 text-xs">
                  Capture assigns a structural Clip Type and records source attribution. Inspectors measure structure, Extractors create searchable representations, Classifiers assign Content Types, and Suggestions offer contextual next steps.
                </p>
                <p className="theme-text-muted mt-2 max-w-3xl text-xs leading-relaxed">
                  Analysis runs in four bounded passes: inspect, extract, classify, and suggest. Each participant runs at most once and only when its declared inputs are available.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-info-text text-xs font-bold">Content Classification</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Enabled Classifiers run locally in priority order; the lowest number runs first. A Classifier uses one or more regular expressions and may add a validator to reduce false positives. Use Settings → Analysis to test samples, manage Content Types, and reset shipped definitions.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Editing a Classifier affects new text clips. <strong>Rescan Clips</strong> explicitly reapplies the current Classifier order and can change Content Types, Smart Bin membership, and sensitive-content masking. Images and files are left unchanged.
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <h4 className="theme-status-info-text text-xs font-bold">Structural inspection</h4>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Structure records content-free facts such as text counts, image dimensions, file item counts, and origin. Clip Preview and the CLI use the same versioned result.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    File availability and total size are live observations. They are checked when displayed and are not stored as durable analysis facts.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    An installed ffprobe or MediaInfo executable also supplies bounded container, codec, stream-count, and duration facts for copied audio and video files. Media metadata is inspected live without returning file paths.
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <Terminal className="h-4 w-4" />
                    <span>Custom Extractors</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    A custom command can turn image data or file references into searchable text through the bounded <code>custom-command-v1</code> protocol.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    New custom commands begin disabled. Review the selected executable before enabling automatic processing for matching clips.
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <AudioLines className="h-4 w-4" />
                    <span>Audio transcription</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    Whisper Transcription uses an installed whisper.cpp executable and a selected local GGML model. M4A and AAC preparation also requires FFmpeg.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Stored transcripts are searchable and do not replace file references. Models are never downloaded automatically.
                  </p>
                </section>

                <section className="theme-panel space-y-3 rounded-xl border p-4">
                  <div className="theme-status-success-text flex items-center gap-2 text-xs font-bold">
                    <ScanText className="h-4 w-4" />
                    <span>Optical Character Recognition</span>
                  </div>
                  <p className="theme-text-main text-xs leading-relaxed">
                    OCR uses Apple Vision on macOS or an installed Tesseract 5 executable to extract searchable text from captured images and screenshots. It does not replace the original image.
                  </p>
                  <p className="theme-text-muted text-xs leading-relaxed">
                    Disabling OCR cancels background work and discards late results while preserving completed text. Re-enabling it resumes eligible backfill when an available image text Extractor is enabled. Check progress under Settings → Analysis or with <code>pasted ocr status --json</code>.
                  </p>
                </section>
              </div>
            </div>
          )}

          {activeSubTab === 'transformations' && (
            <div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Workflow className="w-5 h-5 theme-status-info-text" />
                  <span>Transformations</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  Describe the result once, save it as a Transform, then reuse it wherever text enters or leaves the clipboard workflow.
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
