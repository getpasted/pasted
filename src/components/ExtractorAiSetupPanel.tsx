import { CircleAlert, CircleCheck, Sparkles } from 'lucide-react';

import { translate, type TranslationKey } from '../localization/runtime';
import type { ExtractorDiagnosticCode, ExtractorDiagnosticReport } from './contentExtractorModel';
import { AppDialogButton } from './AppDialogLayout';

const issueKeys: Record<ExtractorDiagnosticCode, TranslationKey> = {
  invalid_recipe: 'component.contentExtractorManagerDialog.diagnostic.invalidRecipe',
  executable_not_configured: 'component.contentExtractorManagerDialog.diagnostic.executableNotConfigured',
  executable_unavailable: 'component.contentExtractorManagerDialog.diagnostic.executableUnavailable',
  resource_not_configured: 'component.contentExtractorManagerDialog.diagnostic.resourceNotConfigured',
  resource_unavailable: 'component.contentExtractorManagerDialog.diagnostic.resourceUnavailable',
};

const issueMessage = (code: ExtractorDiagnosticCode) => translate(issueKeys[code]);

export function ExtractorAiSetupPanel({
  visible,
  hasIntelligence,
  repairing,
  guidanceIncomplete,
  diagnostic,
  setupGuidance,
  onRepair,
  onOpenIntelligence,
}: {
  visible: boolean;
  hasIntelligence: boolean;
  repairing: boolean;
  guidanceIncomplete: boolean;
  diagnostic: ExtractorDiagnosticReport | null;
  setupGuidance: string[];
  onRepair: () => void;
  onOpenIntelligence?: () => void;
}) {
  if (!visible) return null;
  const ready = diagnostic?.isAvailable === true;
  return <section className={`${ready ? 'theme-status-success' : 'theme-status-warning'} space-y-3 rounded-xl border p-3`}>
    <div className="flex items-start gap-2">
      {ready ? <CircleCheck className="mt-0.5 h-4 w-4 shrink-0" /> : <CircleAlert className="mt-0.5 h-4 w-4 shrink-0" />}
      <div className="min-w-0 flex-1">
        <h3 className="font-semibold">{translate('component.contentExtractorManagerDialog.guidedSetup')}</h3>
        <p className="mt-0.5 text-[10px] opacity-80">{translate(ready
          ? 'component.contentExtractorManagerDialog.recipePassesLocalAvailabilityChecks'
          : 'component.contentExtractorManagerDialog.aiCanReviseThisRecipeAndProvideSetupStepsForThisSystem')}</p>
      </div>
    </div>
    {!ready && diagnostic && diagnostic.issues.length > 0 && <ul className="list-disc space-y-1 ps-5 text-[10px]">
      {diagnostic.issues.map((issue) => <li key={`${issue.code}:${issue.subjectId}`}>
        {issue.subjectId !== 'recipe' && <><span className="font-semibold">{issue.label}</span> — </>}
        {issueMessage(issue.code)}
      </li>)}
    </ul>}
    {!ready && setupGuidance.length > 0 && <ol className="list-decimal space-y-1 ps-5 text-[10px]">
      {setupGuidance.map((item) => <li key={item} dir="auto">{item}</li>)}
    </ol>}
    {!ready && diagnostic && guidanceIncomplete && <p className="text-[10px] font-semibold">
      {translate('component.contentExtractorManagerDialog.aiCouldNotProducePreciseSetupSteps')}
    </p>}
    {!ready && <div className="flex justify-end">
      {hasIntelligence
        ? <AppDialogButton onClick={onRepair} disabled={repairing}>
          <Sparkles className="h-3.5 w-3.5" /> {translate(repairing
            ? 'component.contentExtractorManagerDialog.diagnosing'
            : 'component.contentExtractorManagerDialog.diagnoseWithAi')}
        </AppDialogButton>
        : <AppDialogButton onClick={onOpenIntelligence} disabled={!onOpenIntelligence}>
          <Sparkles className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.connectAi')}
        </AppDialogButton>}
    </div>}
  </section>;
}
