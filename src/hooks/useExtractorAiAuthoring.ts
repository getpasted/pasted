import { useState, type Dispatch, type SetStateAction } from 'react';

import { proposeExtractorRecipe, repairExtractorRecipe } from '../extractorAiAuthoring';
import type {
  ExtractorAuthoringManifest,
  ExtractorDiagnosticReport,
  ExtractorInput,
  ExtractorRecipe,
  ExtractorRecipeProposal,
  ExtractorRepairOutcome,
  ExtractorRepairStatus,
} from '../components/contentExtractorModel';
import { useToast } from '../components/ToastProvider';
import { errorMessage } from '../utils/errors';
import { safeInvoke as invoke } from '../utils/tauri';

interface UseExtractorAiAuthoringProps {
  draft: ExtractorInput;
  recipe: ExtractorRecipe;
  prompt: string;
  setDraft: Dispatch<SetStateAction<ExtractorInput>>;
  setRecipe: Dispatch<SetStateAction<ExtractorRecipe>>;
  setAuthoring: Dispatch<SetStateAction<ExtractorAuthoringManifest | null>>;
  setSetupGuidance: Dispatch<SetStateAction<string[]>>;
}

export function useExtractorAiAuthoring({
  draft,
  recipe,
  prompt,
  setDraft,
  setRecipe,
  setAuthoring,
  setSetupGuidance,
}: UseExtractorAiAuthoringProps) {
  const { showToast } = useToast();
  const [generating, setGenerating] = useState(false);
  const [repairing, setRepairing] = useState(false);
  const [diagnostic, setDiagnostic] = useState<ExtractorDiagnosticReport | null>(null);
  const [repairStatus, setRepairStatus] = useState<ExtractorRepairStatus | null>(null);

  const applyProposal = (proposal: ExtractorRecipeProposal | ExtractorRepairOutcome) => {
    setDraft((current) => ({
      ...current,
      name: proposal.name,
      description: proposal.description,
      engine: 'recipe-v1',
      inputContract: proposal.recipe.accepts[0],
      outputContract: proposal.recipe.output,
      executablePath: proposal.recipe.steps[0]?.executable.path ?? null,
      modelPath: proposal.recipe.resources.find((resource) => resource.id === 'model')?.path ?? null,
    }));
    setRecipe(proposal.recipe);
    setAuthoring(proposal.authoring);
    setSetupGuidance(proposal.setupGuidance);
    if ('diagnostic' in proposal) {
      setDiagnostic(proposal.diagnostic);
      setRepairStatus(proposal.status);
    }
  };

  const repair = async (
    candidate = recipe,
    name = draft.name,
    description = draft.description,
  ) => {
    setRepairing(true);
    try {
      const outcome = await repairExtractorRecipe({
        name,
        description,
        recipe: candidate,
        prompt,
      });
      applyProposal(outcome);
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setRepairing(false);
    }
  };

  const generate = async () => {
    if (!prompt.trim()) return;
    setGenerating(true);
    try {
      const proposal = await proposeExtractorRecipe(prompt);
      applyProposal(proposal);
      const report = await invoke<ExtractorDiagnosticReport>('diagnose_content_extractor_recipe', {
        recipe: proposal.recipe,
      });
      setDiagnostic(report);
      if (!report.isAvailable) {
        await repair(proposal.recipe, proposal.name, proposal.description);
      }
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setGenerating(false);
    }
  };

  const clear = () => {
    setAuthoring(null);
    setSetupGuidance([]);
    setDiagnostic(null);
    setRepairStatus(null);
  };

  return { clear, diagnostic, generate, generating, repair, repairing, repairStatus };
}
