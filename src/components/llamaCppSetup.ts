import { translate } from '../localization/runtime';
import { detectDesktopPlatform } from '../utils/platform';

const LLAMA_LABELS_REF = 'extractor:llama-cpp-labels';
const MODEL = 'ggml-org/SmolVLM-500M-Instruct-GGUF';
const DOWNLOAD = `llama-cli -hf ${MODEL} -p "" -n 0 --no-warmup`;

export function llamaCppSetupGuidance(stableRef?: string) {
  if (stableRef !== LLAMA_LABELS_REF) return [];
  const commands = detectDesktopPlatform() === 'windows'
    ? ['winget install llama.cpp']
    : detectDesktopPlatform() === 'macos'
      ? ['brew install llama.cpp']
      : ['brew install llama.cpp', 'conda install -c conda-forge llama.cpp'];
  return [
    ...commands.map((command) => translate(
      'component.contentExtractorManagerDialog.installLlamaCppWithCommand',
      { command },
    )),
    translate('component.contentExtractorManagerDialog.downloadLlamaCppModelWithCommand', {
      command: DOWNLOAD,
    }),
  ];
}
