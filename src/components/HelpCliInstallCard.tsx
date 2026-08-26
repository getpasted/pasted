import { Check, Copy, Download } from 'lucide-react';

import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { CLI_ALIAS_COMMAND, CLI_SYMLINK_COMMAND } from './helpCliCatalog';

interface HelpCliInstallCardProps {
  copiedCmd: string | null;
  onCopyCode: (code: string) => void;
  onInstallCli: () => void;
}

export function HelpCliInstallCard({ copiedCmd, onCopyCode, onInstallCli }: HelpCliInstallCardProps) {
  return (
    <div className="theme-status-info space-y-3 rounded-xl border p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2 text-xs font-bold">
          <Download className="h-4 w-4" />
          <span>{translate('component.helpView.installCliToPath')}</span>
        </div>
        <ActionButton variant="primary" onClick={onInstallCli}>
          <Download className="h-3.5 w-3.5" />
          <span>{translate('component.helpView.value1ClickSymlinkToLocalBin')}</span>
        </ActionButton>
      </div>

      <div className="theme-text-main space-y-2 text-xs">
        <p className="theme-title font-semibold">{translate('component.helpView.manualPathSetup')}</p>
        <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
          <div className="theme-code-surface min-w-0 rounded-lg border p-2.5">
            <div className="mb-2 flex items-center justify-between gap-2">
              <span className="theme-status-success-text text-[10px] font-semibold">{translate('component.helpView.symlinkInUsrLocalBin')}</span>
              <button
                type="button"
                onClick={() => onCopyCode(CLI_SYMLINK_COMMAND)}
                className="theme-icon-button ui-control-radius grid h-7 w-7 shrink-0 place-items-center border"
                title={translate('component.helpView.copyCommand')}
              >
                {copiedCmd === CLI_SYMLINK_COMMAND ? <Check className="theme-status-success-text h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
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
                className="theme-icon-button ui-control-radius grid h-7 w-7 shrink-0 place-items-center border"
                title={translate('component.helpView.copyAlias')}
              >
                {copiedCmd === CLI_ALIAS_COMMAND ? <Check className="theme-status-success-text h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              </button>
            </div>
            <code className="selectable-text block select-text whitespace-pre-wrap break-all font-mono text-[11px]">{CLI_ALIAS_COMMAND}</code>
          </div>
        </div>
      </div>
    </div>
  );
}
