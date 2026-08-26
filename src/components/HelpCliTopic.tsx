import { Check, Copy, Terminal } from 'lucide-react';

import { translate } from '../localization/runtime';
import { HelpCliInstallCard } from './HelpCliInstallCard';
import { CLI_COMMAND_GROUPS } from './helpCliCatalog';

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

              <HelpCliInstallCard copiedCmd={copiedCmd} onCopyCode={onCopyCode} onInstallCli={onInstallCli} />

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
                              className="theme-icon-button ui-control-radius grid h-7 w-7 shrink-0 place-items-center border"
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
