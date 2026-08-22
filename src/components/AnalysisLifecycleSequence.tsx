import { Clipboard, Lightbulb, Radar, RotateCcw, ScanSearch, ScanText, Search, type LucideIcon } from 'lucide-react';

import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';

function AnalysisManagerRow({
  step,
  icon: Icon,
  title,
  description,
  onManage,
}: {
  step: number;
  icon: LucideIcon;
  title: string;
  description: string;
  onManage: () => void;
}) {
  return <section className="theme-divider flex min-h-[49px] items-center justify-between gap-3 border-b p-2 last:border-b-0" aria-label={translate('component.settingsAnalysisPanel.stepTitle', { step, title })}>
    <div className="flex min-w-0 items-center gap-2 px-1">
      <span className="theme-badge grid h-6 w-6 shrink-0 place-items-center rounded-full border text-[10px] font-bold tabular-nums" aria-hidden="true">{step}</span>
      <div className="min-w-0">
        <h3 className="theme-text-main text-xs font-semibold">{title}</h3>
        <p className="theme-text-muted mt-0.5 text-[10px]">{description}</p>
      </div>
    </div>
    <ActionButton aria-label={translate('component.settingsAnalysisPanel.manageTitle', { title })} onClick={onManage} className="h-7 min-h-7 shrink-0 px-2.5">
      <Icon className="h-3.5 w-3.5" /> {translate('component.settingsAnalysisPanel.manage')}
    </ActionButton>
  </section>;
}

export function AnalysisLifecycleSequence({
  contentClassificationEnabled,
  searchEnabled,
  transformationsEnabled,
  restoring,
  onOpenCapture,
  onOpenInspector,
  onOpenExtractor,
  onOpenIndex,
  onOpenClassifier,
  onOpenSuggestion,
  onReset,
  typesEnabled,
}: {
  contentClassificationEnabled: boolean;
  searchEnabled: boolean;
  transformationsEnabled: boolean;
  restoring: boolean;
  onOpenCapture: () => void;
  onOpenInspector: () => void;
  onOpenExtractor: () => void;
  onOpenIndex: () => void;
  onOpenClassifier: () => void;
  onOpenSuggestion: () => void;
  onReset: () => void;
  typesEnabled: boolean;
}) {
  const classificationEnabled = contentClassificationEnabled || typesEnabled;
  return <section className="theme-surface overflow-hidden rounded-xl border" aria-label={translate('component.settingsAnalysisPanel.analysisSequence')}>
    <div>
      <AnalysisManagerRow
        step={1}
        icon={Clipboard}
        title={translate('component.settingsAnalysisPanel.capture')}
        description={translate('component.settingsAnalysisPanel.assignClipTypeAndCaptureContext')}
        onManage={onOpenCapture}
      />
      <AnalysisManagerRow
        step={2}
        icon={ScanSearch}
        title={translate('component.settingsAnalysisPanel.inspect')}
        description={translate('component.settingsAnalysisPanel.measureStructureAndMediaFacts')}
        onManage={onOpenInspector}
      />
      {classificationEnabled && <AnalysisManagerRow
        step={3}
        icon={Radar}
        title={translate('component.settingsAnalysisPanel.classify')}
        description={translate('component.settingsAnalysisPanel.assignRegisteredContentTypesToAnalyzableText')}
        onManage={onOpenClassifier}
      />}
      <AnalysisManagerRow
        step={3 + Number(classificationEnabled)}
        icon={ScanText}
        title={translate('component.settingsAnalysisPanel.extract')}
        description={translate('component.settingsAnalysisPanel.createSearchableRepresentations')}
        onManage={onOpenExtractor}
      />
      {searchEnabled && <AnalysisManagerRow
        step={4 + Number(classificationEnabled)}
        icon={Search}
        title={translate('component.settingsAnalysisPanel.index')}
        description={translate('component.settingsAnalysisPanel.keepCapturedAndExtractedTextReadyForSearch')}
        onManage={onOpenIndex}
      />}
      {transformationsEnabled && <AnalysisManagerRow
        step={4 + Number(searchEnabled) + Number(classificationEnabled)}
        icon={Lightbulb}
        title={translate('component.settingsAnalysisPanel.suggest')}
        description={translate('component.settingsAnalysisPanel.suggestActionsFromAnalysisSignals')}
        onManage={onOpenSuggestion}
      />}
    </div>
    <div className="theme-divider flex items-center justify-between gap-3 border-t px-3 py-2">
      <ActionButton onClick={onReset} disabled={restoring}>
        <RotateCcw className="h-3.5 w-3.5" /> {restoring ? translate('component.settingsAnalysisPanel.resetting') : translate('component.settingsAnalysisPanel.reset')}
      </ActionButton>
      <p className="theme-text-muted text-end text-[10px]">{translate('component.settingsAnalysisPanel.notAllStepsRunForAllClipsSomeStepsMayBeLong')}</p>
    </div>
  </section>;
}
