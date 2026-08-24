import { Sparkles } from 'lucide-react';

import { translate } from '../localization/runtime';
import { AppDialogButton } from './AppDialogLayout';

export function ExtractorAiAuthoringPanel({
  isNew,
  prompt,
  generating,
  hasIntelligence,
  onPromptChange,
  onGenerate,
  onOpenIntelligence,
}: {
  isNew: boolean;
  prompt: string;
  generating: boolean;
  hasIntelligence: boolean;
  onPromptChange: (prompt: string) => void;
  onGenerate: () => void;
  onOpenIntelligence?: () => void;
}) {
  return <div className="theme-subtle-surface space-y-3 rounded-xl border p-3">
    <label className="block space-y-1">
      <span className="theme-text-muted block text-[10px] font-semibold">{translate(isNew
        ? 'component.contentExtractorManagerDialog.describeExtractor'
        : 'component.contentExtractorManagerDialog.describeExtractorRevision')}</span>
      <textarea
        dir="auto"
        value={prompt}
        onChange={(event) => onPromptChange(event.target.value)}
        placeholder={translate(isNew
          ? 'component.contentExtractorManagerDialog.describeExtractorPlaceholder'
          : 'component.contentExtractorManagerDialog.describeExtractorRevisionPlaceholder')}
        className="theme-input ui-field-radius min-h-20 w-full resize-y border px-3 py-2"
      />
    </label>
    <div className="flex flex-wrap items-center justify-between gap-2">
      <span className="theme-text-muted text-[10px]">{translate('component.contentExtractorManagerDialog.aiCreatesLocalReviewableRecipe')}</span>
      {hasIntelligence
        ? <AppDialogButton variant="primary" onClick={onGenerate} disabled={!prompt.trim() || generating}>
          <Sparkles className="h-3.5 w-3.5" /> {generating
            ? translate('component.contentExtractorManagerDialog.creating')
            : translate(isNew ? 'component.contentExtractorManagerDialog.createWithAi' : 'component.contentExtractorManagerDialog.reviseWithAi')}
        </AppDialogButton>
        : <AppDialogButton onClick={onOpenIntelligence} disabled={!onOpenIntelligence}>
          <Sparkles className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.connectAi')}
        </AppDialogButton>}
    </div>
  </div>;
}
