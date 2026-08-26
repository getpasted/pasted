export type SetupPlatform = 'macos' | 'windows' | 'linux' | 'unknown';

export interface ExtractorSetupCommand {
  kind: 'install' | 'download';
  dependency: string;
  command: string;
}

interface SetupRecipe {
  steps: Array<{
    executable: { discover: string[] };
    arguments: string[];
  }>;
  resources: Array<{ required: boolean; path: string | null }>;
}

const installers: Record<string, { name: string; commands: Partial<Record<SetupPlatform, string[]>> }> = {
  'llama-cli': {
    name: 'llama.cpp',
    commands: {
      windows: ['winget install llama.cpp'],
      macos: ['brew install llama.cpp'],
      linux: ['brew install llama.cpp', 'conda install -c conda-forge llama.cpp'],
    },
  },
  tesseract: {
    name: 'Tesseract',
    commands: {
      windows: ['winget install UB-Mannheim.TesseractOCR'],
      macos: ['brew install tesseract'],
      linux: ['sudo apt-get install tesseract-ocr'],
    },
  },
};

export function extractorSetupCommands(
  recipe: SetupRecipe,
  platform: SetupPlatform,
): ExtractorSetupCommand[] {
  if (recipe.resources.some(({ required, path }) => required && !path)) return [];
  return recipe.steps.flatMap((step) => {
    const executable = step.executable.discover.find((name) => installers[name]);
    if (!executable) return [];
    const installer = installers[executable];
    const install = (installer.commands[platform] ?? []).map((command) => ({
      kind: 'install' as const,
      dependency: installer.name,
      command,
    }));
    const repositoryIndex = step.arguments.indexOf('-hf');
    const repository = repositoryIndex >= 0 ? step.arguments[repositoryIndex + 1] : undefined;
    const download = repository ? [{
      kind: 'download' as const,
      dependency: installer.name,
      command: `${executable} -hf ${repository} -p "" -n 0 --no-warmup`,
    }] : [];
    return [...install, ...download];
  });
}
