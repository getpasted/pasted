import { useState } from 'react';
import { Check, CircleAlert, Copy } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { ExtractorCommandSetup } from './extractorCommandSetup';
import { AppDialogButton } from './AppDialogLayout';

export function ExtractorCommandSetupPanel({ setup }: { setup: ExtractorCommandSetup }) {
  const [copied, setCopied] = useState<string | null>(null);
  const copy = async (command: string) => {
    await navigator.clipboard.writeText(command);
    setCopied(command);
    window.setTimeout(() => setCopied((current) => current === command ? null : current), 1_500);
  };

  return <section className="theme-status-warning space-y-3 rounded-xl border p-3">
    <div className="flex items-start gap-2">
      <CircleAlert className="mt-0.5 h-4 w-4 shrink-0" />
      <div className="min-w-0 flex-1">
        <h3 className="font-semibold">{translate('component.contentExtractorManagerDialog.guidedSetup')}</h3>
        <p className="mt-0.5 text-[10px] opacity-80">
          {translate('component.contentExtractorManagerDialog.runEachCommandThenCheckAvailabilityAgain')}
        </p>
      </div>
    </div>
    <ol className="list-decimal space-y-3 ps-5 text-[10px]">
      {setup.steps.map(({ label, command }) => <li key={command}>
        <p className="mb-1 font-semibold">{label}</p>
        <div className="theme-input flex min-w-0 items-center rounded-lg border p-1 ps-2">
          <code dir="ltr" className="min-w-0 flex-1 select-text overflow-x-auto whitespace-nowrap font-mono">
            {command}
          </code>
          <AppDialogButton
            type="button"
            className="ms-2 h-7 min-h-7 shrink-0 px-2"
            onClick={() => void copy(command)}
            title={copied === command ? translate('action.copied') : translate('action.copy')}
            aria-label={copied === command ? translate('action.copied') : translate('action.copy')}
          >
            {copied === command ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </AppDialogButton>
        </div>
      </li>)}
    </ol>
  </section>;
}
