import type {
  ExtractorRecipe,
  ExtractorRecipeProposal,
  ExtractorRepairOutcome,
} from './components/contentExtractorModel';
import { safeInvoke as invoke } from './utils/tauri';

export function proposeExtractorRecipe(prompt: string) {
  return invoke<ExtractorRecipeProposal>('propose_extractor_recipe', {
    request: { prompt },
  });
}

export function repairExtractorRecipe({
  name,
  description,
  recipe,
  prompt,
}: {
  name: string;
  description: string;
  recipe: ExtractorRecipe;
  prompt?: string;
}) {
  return invoke<ExtractorRepairOutcome>('repair_extractor_recipe', {
    request: { name, description, recipe, prompt: prompt?.trim() || null, maxAttempts: 3 },
  });
}
