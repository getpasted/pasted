import { Check, Copy, Download, Terminal } from 'lucide-react';

import { translate } from '../localization/runtime';
import { CLI_ALIAS_COMMAND, CLI_COMMAND_GROUPS, CLI_SYMLINK_COMMAND } from './helpCliCatalog';

interface HelpCliTopicProps {
  copiedCmd: string | null;
  onCopyCode: (code: string) => void;
  onInstallCli: () => void;
}

export function HelpCliTopic({ copiedCmd, onCopyCode, onInstallCli }: HelpCliTopicProps) {
  return (
<div className="space-y-6 animate-in fade-in">
              <div>
                <h3 className="theme-title text-lg font-bold flex items-center space-x-2">
                  <Terminal className="w-5 h-5 theme-status-info-text" />
                  <span>{translate('component.helpView.terminalCliCommand', { command: 'pasted' })}</span>
                </h3>
                <p className="theme-text-muted text-xs mt-1">
                  {translate('component.helpView.theStandaloneNativeCommandLineToolCanPipeDataIntoClipboardHistory')}
                </p>
              </div>

              {/* PATH Installation Box */}
              <div className="theme-status-info p-4 rounded-xl border space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2 text-xs font-bold">
                    <Download className="w-4 h-4" />
                    <span>{translate('component.helpView.installCliToPath')}</span>
                  </div>
                  <button
                    onClick={onInstallCli}
                    className="theme-primary-button ui-control-radius flex items-center space-x-1.5 px-3 py-1.5 border text-xs font-bold transition-colors cursor-pointer shadow-sm"
                  >
                    <Download className="w-3.5 h-3.5" />
                    <span>{translate('component.helpView.value1ClickSymlinkToLocalBin')}</span>
                  </button>
                </div>

                <div className="theme-text-main space-y-2 text-xs">
                  <p className="font-semibold theme-title">{translate('component.helpView.manualPathSetup')}</p>
                  <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">{translate('component.helpView.symlinkInUsrLocalBin')}</span>
                        <button
                          type="button"
                          onClick={() => onCopyCode(CLI_SYMLINK_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title={translate('component.helpView.copyCommand')}
                        >
                          {copiedCmd === CLI_SYMLINK_COMMAND ? <Check className="h-3.5 w-3.5 theme-status-success-text" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_SYMLINK_COMMAND}</code>
                    </div>

                    <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="theme-status-success-text text-[10px] font-semibold">{translate('component.helpView.shellAlias')}</span>
                        <button
                          type="button"
                          onClick={() => onCopyCode(CLI_ALIAS_COMMAND)}
                          className="theme-icon-button shrink-0 rounded border p-1"
                          title={translate('component.helpView.copyAlias')}
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
                  <h4 className="theme-title text-sm font-bold">{translate('component.helpView.commandReference')}</h4>
                  <p className="theme-text-muted mt-1 text-xs">
                    {translate('component.helpView.commandReferenceDescription', { flag: '--json' })}</p>
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
                              onClick={() => onCopyCode(command.usage)}
                              className="theme-icon-button shrink-0 rounded border p-1.5"
                              title={translate('component.helpView.copyCommand')}
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
  );
}
