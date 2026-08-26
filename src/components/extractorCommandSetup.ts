import { translate } from '../localization/runtime';
import { detectDesktopPlatform } from '../utils/platform';
import type { ExtractorRecipe } from './contentExtractorModel';
import { extractorSetupCommands, type SetupPlatform } from './extractorCommandSetupModel';

export interface ExtractorCommandSetup {
  steps: Array<{ label: string; command: string }>;
}

export function extractorCommandSetup(
  recipe: ExtractorRecipe,
  platform: SetupPlatform = detectDesktopPlatform(),
): ExtractorCommandSetup | null {
  const steps = extractorSetupCommands(recipe, platform).map(({ kind, dependency, command }) => ({
    label: kind === 'install'
      ? translate('component.contentExtractorManagerDialog.installDependency', { name: dependency })
      : translate('component.contentExtractorManagerDialog.downloadConfiguredModel'),
    command,
  }));
  return steps.length > 0 ? { steps } : null;
}
