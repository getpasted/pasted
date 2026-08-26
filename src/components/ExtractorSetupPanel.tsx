import type { ExtractorDiagnosticReport, ExtractorRecipe } from './contentExtractorModel';
import { ExtractorAiSetupPanel } from './ExtractorAiSetupPanel';
import { ExtractorCommandSetupPanel } from './ExtractorCommandSetupPanel';
import { extractorCommandSetup } from './extractorCommandSetup';

export function ExtractorSetupPanel({
  recipe,
  visible,
  hasIntelligence,
  repairing,
  guidanceIncomplete,
  diagnostic,
  setupGuidance,
  onRepair,
  onOpenIntelligence,
}: {
  recipe: ExtractorRecipe;
  visible: boolean;
  hasIntelligence: boolean;
  repairing: boolean;
  guidanceIncomplete: boolean;
  diagnostic: ExtractorDiagnosticReport | null;
  setupGuidance: string[];
  onRepair: () => void;
  onOpenIntelligence?: () => void;
}) {
  const commandSetup = extractorCommandSetup(recipe);
  if (visible && commandSetup) return <ExtractorCommandSetupPanel setup={commandSetup} />;
  return <ExtractorAiSetupPanel
    visible={visible}
    hasIntelligence={hasIntelligence}
    repairing={repairing}
    guidanceIncomplete={guidanceIncomplete}
    diagnostic={diagnostic}
    setupGuidance={setupGuidance}
    onRepair={onRepair}
    onOpenIntelligence={onOpenIntelligence}
  />;
}
