import type { IntelligenceProviderKind } from '../types';

export interface IntelligenceProviderOption {
  value: IntelligenceProviderKind;
  label: string;
  endpoint: string;
  model: string;
  local: boolean;
}

export const INTELLIGENCE_PROVIDERS: IntelligenceProviderOption[] = [
  { value: 'ollama', label: 'Ollama', endpoint: 'http://127.0.0.1:11434', model: '', local: true },
  { value: 'lm_studio', label: 'LM Studio', endpoint: 'http://127.0.0.1:1234/v1', model: '', local: true },
  { value: 'openai_compatible', label: 'OpenAI-compatible', endpoint: 'https://api.openai.com/v1', model: '', local: false },
  { value: 'anthropic', label: 'Anthropic', endpoint: 'https://api.anthropic.com', model: '', local: false },
  { value: 'gemini', label: 'Gemini', endpoint: 'https://generativelanguage.googleapis.com', model: '', local: false },
  { value: 'cli', label: 'CLI adapter', endpoint: '', model: '', local: true },
];

export function intelligenceProviderLabel(kind: IntelligenceProviderKind) {
  return INTELLIGENCE_PROVIDERS.find((provider) => provider.value === kind)?.label ?? kind;
}
