import type { ManualTransformStep } from '../types';

export interface ManualTransformEditorStep {
  id: string;
  operation_ref: string;
  config?: string | null;
  findPattern?: string;
  replacePattern?: string;
  matchMode?: 'regex' | 'literal' | 'wildcard';
  caseSensitive?: boolean;
  tagName?: string;
  shellCommand?: string;
  quoteBefore?: string;
  quoteAfter?: string;
  applyToEachLine?: boolean;
}

export function operationTypeForRef(operationRef: string) {
  return operationRef.startsWith('builtin:') ? operationRef.slice('builtin:'.length) : null;
}

function parseConfig(step: ManualTransformStep, operationType: string | null) {
  if ((operationType !== 'regex' && operationType !== 'quote_text') || !step.configJson) return {};
  try {
    return JSON.parse(step.configJson) as Record<string, unknown>;
  } catch {
    return {};
  }
}

export function pipelineStepToEditorStep(
  step: ManualTransformStep,
  index: number,
): ManualTransformEditorStep {
  const operationType = operationTypeForRef(step.operationRef);
  const parsedConfig = parseConfig(step, operationType);
  return {
    id: `step-${index}-${Date.now()}`,
    operation_ref: step.operationRef,
    config: step.configJson,
    findPattern: typeof parsedConfig.pattern === 'string' ? parsedConfig.pattern : '',
    replacePattern: typeof parsedConfig.replacement === 'string' ? parsedConfig.replacement : '',
    matchMode: parsedConfig.matchMode === 'literal' || parsedConfig.matchMode === 'wildcard'
      ? parsedConfig.matchMode
      : 'regex',
    caseSensitive: parsedConfig.caseSensitive === true,
    tagName: operationType === 'wrap_tags' ? step.configJson || 'code' : 'code',
    shellCommand: operationType === 'shell_script' ? step.configJson || 'cat' : 'cat',
    quoteBefore: typeof parsedConfig.before === 'string' ? parsedConfig.before : '> ',
    quoteAfter: typeof parsedConfig.after === 'string' ? parsedConfig.after : '',
    applyToEachLine: typeof parsedConfig.applyToEachLine === 'boolean'
      ? parsedConfig.applyToEachLine
      : true,
  };
}

export function createDefaultManualTransformStep(): ManualTransformEditorStep {
  return {
    id: `step-${Date.now()}-${Math.random()}`,
    operation_ref: 'builtin:smart_punctuation',
    config: null,
    findPattern: '',
    replacePattern: '',
    matchMode: 'regex',
    caseSensitive: false,
    tagName: 'code',
    shellCommand: 'tr "a-z" "A-Z"',
    quoteBefore: '> ',
    quoteAfter: '',
    applyToEachLine: true,
  };
}

export function compileManualTransformStep(step: ManualTransformEditorStep) {
  const operationType = operationTypeForRef(step.operation_ref);
  let configJson: string | null = step.config || null;
  if (operationType === 'regex') {
    configJson = JSON.stringify({
      pattern: step.findPattern || '',
      replacement: step.replacePattern || '',
      matchMode: step.matchMode || 'regex',
      caseSensitive: step.caseSensitive || false,
    });
  } else if (operationType === 'wrap_tags') {
    configJson = step.tagName || 'code';
  } else if (operationType === 'shell_script') {
    configJson = step.shellCommand || 'cat';
  } else if (operationType === 'quote_text') {
    configJson = JSON.stringify({
      before: step.quoteBefore ?? '> ',
      after: step.quoteAfter ?? '',
      applyToEachLine: step.applyToEachLine ?? true,
    });
  }
  return {
    operationRef: step.operation_ref,
    configJson,
    failurePolicy: 'stop' as const,
  };
}
